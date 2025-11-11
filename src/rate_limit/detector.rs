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

use crate::rate_limit::TierConfig;
use reqwest::header::HeaderMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Detects and caches rate limits from API response headers.
///
/// This detector parses provider-specific rate limit headers and caches
/// the detected configuration for reuse. Each provider has different
/// header formats, so provider-specific detection methods are provided.
///
/// # Example
///
/// ```rust,ignore
/// use boticelli::HeaderRateLimitDetector;
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
    pub fn new() -> Self {
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
    pub async fn detect_gemini(&self, headers: &HeaderMap) -> Option<TierConfig> {
        // Parse rate limit headers
        let rpm = parse_header_u32(headers, "x-ratelimit-limit")?;

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
    pub async fn detect_anthropic(&self, headers: &HeaderMap) -> Option<TierConfig> {
        let rpm = parse_header_u32(headers, "anthropic-ratelimit-requests-limit")?;
        let tpm = parse_header_u64(headers, "anthropic-ratelimit-tokens-limit")?;

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
            rpd: None, // Anthropic doesn't have daily limits
            max_concurrent: Some(5), // Not exposed in headers
            daily_quota_usd: None,
            cost_per_million_input_tokens: Some(3.0), // Varies by model
            cost_per_million_output_tokens: Some(15.0),
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
    pub async fn detect_openai(&self, headers: &HeaderMap) -> Option<TierConfig> {
        let rpm = parse_header_u32(headers, "x-ratelimit-limit-requests")?;
        let tpm = parse_header_u64(headers, "x-ratelimit-limit-tokens")?;

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
    pub async fn get_cached(&self) -> Option<TierConfig> {
        self.detected_limits.read().await.clone()
    }

    /// Clear the cached detected limits.
    ///
    /// Useful when you want to force fresh detection on the next API call.
    pub async fn clear_cache(&self) {
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
    headers
        .get(key)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}

/// Helper to parse u64 from header value.
fn parse_header_u64(headers: &HeaderMap, key: &str) -> Option<u64> {
    headers
        .get(key)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

    fn create_headers(entries: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (key, value) in entries {
            headers.insert(
                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        headers
    }

    #[cfg(feature = "gemini")]
    #[tokio::test]
    async fn test_detect_gemini_free_tier() {
        let detector = HeaderRateLimitDetector::new();
        let headers = create_headers(&[
            ("x-ratelimit-limit", "10"),
            ("x-ratelimit-remaining", "9"),
            ("x-ratelimit-reset", "1705012345"),
        ]);

        let config = detector.detect_gemini(&headers).await.unwrap();
        assert_eq!(config.name, "Free");
        assert_eq!(config.rpm, Some(10));
        assert_eq!(config.tpm, Some(250_000));
        assert_eq!(config.rpd, Some(250));
        assert_eq!(config.max_concurrent, Some(1));
    }

    #[cfg(feature = "gemini")]
    #[tokio::test]
    async fn test_detect_gemini_payasyougo_tier() {
        let detector = HeaderRateLimitDetector::new();
        let headers = create_headers(&[
            ("x-ratelimit-limit", "360"),
            ("x-ratelimit-remaining", "350"),
        ]);

        let config = detector.detect_gemini(&headers).await.unwrap();
        assert_eq!(config.name, "Pay-as-you-go");
        assert_eq!(config.rpm, Some(360));
        assert_eq!(config.tpm, Some(4_000_000));
        assert_eq!(config.rpd, None);
    }

    #[cfg(feature = "anthropic")]
    #[tokio::test]
    async fn test_detect_anthropic_tier1() {
        let detector = HeaderRateLimitDetector::new();
        let headers = create_headers(&[
            ("anthropic-ratelimit-requests-limit", "5"),
            ("anthropic-ratelimit-requests-remaining", "4"),
            ("anthropic-ratelimit-tokens-limit", "20000"),
            ("anthropic-ratelimit-tokens-remaining", "18000"),
        ]);

        let config = detector.detect_anthropic(&headers).await.unwrap();
        assert_eq!(config.name, "Tier 1");
        assert_eq!(config.rpm, Some(5));
        assert_eq!(config.tpm, Some(20_000));
        assert_eq!(config.max_concurrent, Some(5));
    }

    #[cfg(feature = "anthropic")]
    #[tokio::test]
    async fn test_detect_anthropic_tier4() {
        let detector = HeaderRateLimitDetector::new();
        let headers = create_headers(&[
            ("anthropic-ratelimit-requests-limit", "2000"),
            ("anthropic-ratelimit-tokens-limit", "160000"),
        ]);

        let config = detector.detect_anthropic(&headers).await.unwrap();
        assert_eq!(config.name, "Tier 4");
        assert_eq!(config.rpm, Some(2000));
        assert_eq!(config.tpm, Some(160_000));
    }

    #[tokio::test]
    async fn test_detect_openai_free_tier() {
        let detector = HeaderRateLimitDetector::new();
        let headers = create_headers(&[
            ("x-ratelimit-limit-requests", "3"),
            ("x-ratelimit-limit-tokens", "40000"),
            ("x-ratelimit-remaining-requests", "2"),
            ("x-ratelimit-remaining-tokens", "35000"),
        ]);

        let config = detector.detect_openai(&headers).await.unwrap();
        assert_eq!(config.name, "Free");
        assert_eq!(config.rpm, Some(3));
        assert_eq!(config.tpm, Some(40_000));
        assert_eq!(config.rpd, Some(200));
    }

    #[tokio::test]
    async fn test_detect_openai_tier5() {
        let detector = HeaderRateLimitDetector::new();
        let headers = create_headers(&[
            ("x-ratelimit-limit-requests", "10000"),
            ("x-ratelimit-limit-tokens", "100000000"),
        ]);

        let config = detector.detect_openai(&headers).await.unwrap();
        assert_eq!(config.name, "Tier 5");
        assert_eq!(config.rpm, Some(10000));
        assert_eq!(config.tpm, Some(100_000_000));
        assert_eq!(config.rpd, None);
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let detector = HeaderRateLimitDetector::new();

        // Initially empty
        assert!(detector.get_cached().await.is_none());

        // Detect and cache
        let headers = create_headers(&[
            ("x-ratelimit-limit-requests", "500"),
            ("x-ratelimit-limit-tokens", "200000"),
        ]);
        detector.detect_openai(&headers).await;

        // Should be cached
        let cached = detector.get_cached().await.unwrap();
        assert_eq!(cached.name, "Tier 1");

        // Clear cache
        detector.clear_cache().await;
        assert!(detector.get_cached().await.is_none());
    }

    #[tokio::test]
    async fn test_missing_headers_returns_none() {
        let detector = HeaderRateLimitDetector::new();
        let headers = HeaderMap::new();

        assert!(detector.detect_openai(&headers).await.is_none());
    }
}
