//! Tests for header-based rate limit detection.

use botticelli_error::{HttpError, HttpErrorKind, BotticelliResult};
use botticelli_interface::Tier;
use botticelli_rate_limit::HeaderRateLimitDetector;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

fn create_headers(entries: &[(&str, &str)]) -> Result<HeaderMap, HttpError> {
    let mut headers = HeaderMap::new();
    for (key, value) in entries {
        let header_name = HeaderName::from_bytes(key.as_bytes())
            .map_err(|e| HttpError::new(HttpErrorKind::Message(e.to_string())))?;
        let header_value = HeaderValue::from_str(value)
            .map_err(|e| HttpError::new(HttpErrorKind::Message(e.to_string())))?;
        headers.insert(header_name, header_value);
    }
    Ok(headers)
}

#[cfg(feature = "gemini")]
#[tokio::test]
async fn test_detect_gemini_free_tier() -> BotticelliResult<()> {
    let detector = HeaderRateLimitDetector::new();
    let headers = create_headers(&[
        ("x-ratelimit-limit", "10"),
        ("x-ratelimit-remaining", "9"),
        ("x-ratelimit-reset", "1705012345"),
    ])?;

    let config = detector
        .detect_gemini(&headers)
        .await?
        .ok_or_else(|| botticelli_error::RateLimitError::from(botticelli_error::RateLimitErrorKind::Config(
            "Failed to detect config".to_string())
        ))?;

    assert_eq!(config.name(), "Free");
    assert_eq!(config.rpm(), Some(10));
    assert_eq!(config.tpm(), Some(250_000));
    assert_eq!(config.rpd(), Some(250));
    assert_eq!(config.max_concurrent(), Some(1));
    Ok(())
}

#[cfg(feature = "gemini")]
#[tokio::test]
async fn test_detect_gemini_payasyougo_tier() -> BotticelliResult<()> {
    let detector = HeaderRateLimitDetector::new();
    let headers = create_headers(&[
        ("x-ratelimit-limit", "360"),
        ("x-ratelimit-remaining", "350"),
    ])?;

    let config = detector
        .detect_gemini(&headers)
        .await?
        .ok_or_else(|| botticelli_error::RateLimitError::from(botticelli_error::RateLimitErrorKind::Config(
            "Failed to detect config".to_string())
        ))?;

    assert_eq!(config.name(), "Pay-as-you-go");
    assert_eq!(config.rpm(), Some(360));
    assert_eq!(config.tpm(), Some(4_000_000));
    assert_eq!(config.rpd(), None);
    Ok(())
}

#[cfg(feature = "anthropic")]
#[tokio::test]
async fn test_detect_anthropic_tier1() -> BotticelliResult<()> {
    let detector = HeaderRateLimitDetector::new();
    let headers = create_headers(&[
        ("anthropic-ratelimit-requests-limit", "5"),
        ("anthropic-ratelimit-requests-remaining", "4"),
        ("anthropic-ratelimit-tokens-limit", "20000"),
        ("anthropic-ratelimit-tokens-remaining", "18000"),
    ])?;

    let config = detector
        .detect_anthropic(&headers)
        .await?
        .ok_or_else(|| botticelli_error::RateLimitError::from(botticelli_error::RateLimitErrorKind::Config(
            "Failed to detect config".to_string())
        ))?;

    assert_eq!(config.name(), "Tier 1");
    assert_eq!(config.rpm(), Some(5));
    assert_eq!(config.tpm(), Some(20_000));
    assert_eq!(config.max_concurrent(), Some(5));
    Ok(())
}

#[cfg(feature = "anthropic")]
#[tokio::test]
async fn test_detect_anthropic_tier4() -> BotticelliResult<()> {
    let detector = HeaderRateLimitDetector::new();
    let headers = create_headers(&[
        ("anthropic-ratelimit-requests-limit", "2000"),
        ("anthropic-ratelimit-tokens-limit", "160000"),
    ])?;

    let config = detector
        .detect_anthropic(&headers)
        .await?
        .ok_or_else(|| botticelli_error::RateLimitError::from(botticelli_error::RateLimitErrorKind::Config(
            "Failed to detect config".to_string())
        ))?;

    assert_eq!(config.name(), "Tier 4");
    assert_eq!(config.rpm(), Some(2000));
    assert_eq!(config.tpm(), Some(160_000));
    Ok(())
}

#[tokio::test]
async fn test_detect_openai_free_tier() -> BotticelliResult<()> {
    let detector = HeaderRateLimitDetector::new();
    let headers = create_headers(&[
        ("x-ratelimit-limit-requests", "3"),
        ("x-ratelimit-limit-tokens", "40000"),
        ("x-ratelimit-remaining-requests", "2"),
        ("x-ratelimit-remaining-tokens", "35000"),
    ])?;

    let config = detector
        .detect_openai(&headers)
        .await?
        .ok_or_else(|| botticelli_error::RateLimitError::from(botticelli_error::RateLimitErrorKind::Config(
            "Failed to detect config".to_string())
        ))?;

    assert_eq!(config.name(), "Free");
    assert_eq!(config.rpm(), Some(3));
    assert_eq!(config.tpm(), Some(40_000));
    assert_eq!(config.rpd(), Some(200));
    Ok(())
}

#[tokio::test]
async fn test_detect_openai_tier5() -> BotticelliResult<()> {
    let detector = HeaderRateLimitDetector::new();
    let headers = create_headers(&[
        ("x-ratelimit-limit-requests", "10000"),
        ("x-ratelimit-limit-tokens", "100000000"),
    ])?;

    let config = detector
        .detect_openai(&headers)
        .await?
        .ok_or_else(|| botticelli_error::RateLimitError::from(botticelli_error::RateLimitErrorKind::Config(
            "Failed to detect config".to_string())
        ))?;

    assert_eq!(config.name(), "Tier 5");
    assert_eq!(config.rpm(), Some(10000));
    assert_eq!(config.tpm(), Some(100_000_000));
    assert_eq!(config.rpd(), None);
    Ok(())
}

#[tokio::test]
async fn test_cache_functionality() -> BotticelliResult<()> {
    let detector = HeaderRateLimitDetector::new();

    // Initially empty
    assert!(detector.get_cached().await.is_none());

    // Detect and cache
    let headers = create_headers(&[
        ("x-ratelimit-limit-requests", "500"),
        ("x-ratelimit-limit-tokens", "200000"),
    ])?;

    detector.detect_openai(&headers).await?;

    // Should be cached
    let cached = detector.get_cached().await.unwrap();
    assert_eq!(cached.name(), "Tier 1");

    // Clear cache
    detector.clear_cache().await;
    assert!(detector.get_cached().await.is_none());
    Ok(())
}

#[tokio::test]
async fn test_missing_headers_returns_none() -> BotticelliResult<()> {
    let detector = HeaderRateLimitDetector::new();
    let headers = HeaderMap::new();

    assert!(detector
        .detect_openai(&headers)
        .await?
        .is_none());
    Ok(())
}
