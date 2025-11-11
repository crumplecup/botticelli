//! Tests for header-based rate limit detection.

use boticelli::HeaderRateLimitDetector;
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
