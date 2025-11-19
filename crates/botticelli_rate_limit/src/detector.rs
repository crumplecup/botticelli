//! Auto-detection of rate limits from API response headers.
//!
//! This module provides the `HeaderRateLimitDetector` which extracts rate limit
//! information from HTTP response headers and caches detected limits. Different
//! providers use different header conventions, so provider-specific detection
//! methods are provided.
//!
//! Header detection provides the most accurate rate limit information since it:
//! - Reflects your actual current limits (not stale config values)
//! - Automatically updates when you upgrade tiers
//! - Comes directly from the provider

use crate::TierConfig;
use reqwest::header::HeaderMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, instrument};

/// Detects and caches rate limits from API response headers.
///
/// This detector parses provider-specific rate limit headers and caches
/// the detected configuration for reuse. Each provider has different
/// header formats, so provider-specific detection methods are provided.
///
/// # Example
///
/// ```rust,ignore
/// use botticelli_rate_limit::HeaderRateLimitDetector;
///
/// let detector = HeaderRateLimitDetector::new();
///
/// // After making an API call
/// if let Some(tier_config) = detector.detect_gemini(response.headers()).await {
///     println!("Detected tier: {}", tier_config.name);
///     println!("RPM limit: {:?}", tier_config.rpm);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct HeaderRateLimitDetector {
    /// Cached detected limits (updated on each API call)
    detected_limits: Arc<RwLock<Option<TierConfig>>>,
}

impl HeaderRateLimitDetector {
    /// Create a new header rate limit detector.
    #[instrument]
    pub fn new() -> Self {
        debug!("Creating new header rate limit detector");
        Self {
            detected_limits: Arc::new(RwLock::new(None)),
        }
    }

    /// Detect rate limits from Gemini/Google AI response headers.
    ///
    /// Gemini uses standard rate limit headers:
    /// - `x-ratelimit-limit`: Requests allowed in current window
    /// - `x-ratelimit-remaining`: Requests remaining
    /// - `x-ratelimit-reset`: Unix timestamp when limit resets
    ///
    /// Since Gemini doesn't expose TPM/RPD in headers, we infer them from RPM.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = detector.detect_gemini(response.headers()).await;
    /// ```
    #[cfg(feature = "gemini")]
    #[instrument(skip(self, headers))]
    pub async fn detect_gemini(&self, headers: &HeaderMap) -> Option<TierConfig> {
        debug!("Detecting Gemini rate limits from headers");
        
        // Parse rate limit headers
        let rpm = parse_header_u32(headers, "x-ratelimit-limit")?;
        debug!(rpm, "Detected RPM from x-ratelimit-limit header");

        // Gemini doesn't expose TPM/RPD in headers, so we infer from RPM
        let (tpm, rpd, tier_name) = if rpm <= 10 {
            (Some(250_000), Some(250), "Free")
        } else if rpm <= 360 {
            (Some(4_000_000), None, "Pay-as-you-go")
        } else {
            (None, None, "Unknown")
        };

        let config = TierConfig {
            name: tier_name.to_string(),
            rpm: Some(rpm),
            tpm,
            rpd,
            max_concurrent: Some(1), // Gemini doesn't expose this in headers
            daily_quota_usd: None,
            cost_per_million_input_tokens: if rpm <= 10 { Some(0.0) } else { Some(0.075) },
            cost_per_million_output_tokens: if rpm <= 10 { Some(0.0) } else { Some(0.30) },
            models: HashMap::new(), // Header-detected configs don't have model-specific overrides
        };

        // Cache for future use
        *self.detected_limits.write().await = Some(config.clone());

        Some(config)
    }

    /// Detect rate limits from Anthropic response headers.
    ///
    /// Anthropic uses prefixed headers:
    /// - `anthropic-ratelimit-requests-limit`: RPM limit
    /// - `anthropic-ratelimit-requests-remaining`: RPM remaining
    /// - `anthropic-ratelimit-requests-reset`: RPM reset time
    /// - `anthropic-ratelimit-tokens-limit`: TPM limit
    /// - `anthropic-ratelimit-tokens-remaining`: TPM remaining
    /// - `anthropic-ratelimit-tokens-reset`: TPM reset time
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = detector.detect_anthropic(response.headers()).await;
    /// ```
    #[cfg(feature = "anthropic")]
    #[instrument(skip(self, headers))]
    pub async fn detect_anthropic(&self, headers: &HeaderMap) -> Option<TierConfig> {
        debug!("Detecting Anthropic rate limits from headers");
        
        let rpm = parse_header_u32(headers, "anthropic-ratelimit-requests-limit")?;
        let tpm = parse_header_u64(headers, "anthropic-ratelimit-tokens-limit")?;
        debug!(rpm, tpm, "Detected Anthropic rate limits");

        // Determine tier name from limits
        let tier_name = match (rpm, tpm) {
            (5, 20_000) => "Tier 1",
            (50, 40_000) => "Tier 2",
            (1000, 80_000) => "Tier 3",
            (2000, 160_000) => "Tier 4",
            _ => "Custom",
        };

        let config = TierConfig {
            name: tier_name.to_string(),
            rpm: Some(rpm),
            tpm: Some(tpm),
            rpd: None,               // Anthropic doesn't have daily limits
            max_concurrent: Some(5), // Not exposed in headers
            daily_quota_usd: None,
            cost_per_million_input_tokens: Some(3.0), // Varies by model
            cost_per_million_output_tokens: Some(15.0),
            models: HashMap::new(), // Header-detected configs don't have model-specific overrides
        };

        *self.detected_limits.write().await = Some(config.clone());

        Some(config)
    }

    /// Detect rate limits from OpenAI response headers.
    ///
    /// OpenAI uses detailed rate limit headers:
    /// - `x-ratelimit-limit-requests`: RPM limit
    /// - `x-ratelimit-limit-tokens`: TPM limit
    /// - `x-ratelimit-remaining-requests`: RPM remaining
    /// - `x-ratelimit-remaining-tokens`: TPM remaining
    /// - `x-ratelimit-reset-requests`: RPM reset time (duration string)
    /// - `x-ratelimit-reset-tokens`: TPM reset time (duration string)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = detector.detect_openai(response.headers()).await;
    /// ```
    #[instrument(skip(self, headers))]
    pub async fn detect_openai(&self, headers: &HeaderMap) -> Option<TierConfig> {
        debug!("Detecting OpenAI rate limits from headers");
        
        let rpm = parse_header_u32(headers, "x-ratelimit-limit-requests")?;
        let tpm = parse_header_u64(headers, "x-ratelimit-limit-tokens")?;
        debug!(rpm, tpm, "Detected OpenAI rate limits");

        // Determine tier from limits
        let (tier_name, rpd) = match (rpm, tpm) {
            (3, 40_000) => ("Free", Some(200)),
            (500, 200_000) => ("Tier 1", None),
            (5000, 2_000_000) => ("Tier 2", None),
            (10000, 10_000_000) => ("Tier 3", None),
            (10000, 30_000_000) => ("Tier 4", None),
            (10000, 100_000_000) => ("Tier 5", None),
            _ => ("Custom", None),
        };

        let config = TierConfig {
            name: tier_name.to_string(),
            rpm: Some(rpm),
            tpm: Some(tpm),
            rpd,
            max_concurrent: Some(50),
            daily_quota_usd: None,
            cost_per_million_input_tokens: Some(2.50), // Varies by model
            cost_per_million_output_tokens: Some(10.0),
            models: HashMap::new(), // Header-detected configs don't have model-specific overrides
        };

        *self.detected_limits.write().await = Some(config.clone());

        Some(config)
    }

    /// Get last detected limits from cache.
    ///
    /// Returns the most recently detected tier configuration, or None
    /// if no detection has been performed yet.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(cached) = detector.get_cached().await {
    ///     println!("Last detected tier: {}", cached.name);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn get_cached(&self) -> Option<TierConfig> {
        let cached = self.detected_limits.read().await.clone();
        debug!(has_cached = cached.is_some(), "Retrieving cached rate limits");
        cached
    }

    /// Clear the cached detected limits.
    ///
    /// Useful when you want to force fresh detection on the next API call.
    #[instrument(skip(self))]
    pub async fn clear_cache(&self) {
        debug!("Clearing cached rate limits");
        *self.detected_limits.write().await = None;
    }
}

impl Default for HeaderRateLimitDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to parse u32 from header value.
fn parse_header_u32(headers: &HeaderMap, key: &str) -> Option<u32> {
    headers.get(key)?.to_str().ok()?.parse().ok()
}

/// Helper to parse u64 from header value.
fn parse_header_u64(headers: &HeaderMap, key: &str) -> Option<u64> {
    headers.get(key)?.to_str().ok()?.parse().ok()
}
