use botticelli_models::{GeminiModel, GroqModel, ModelId, RateLimitStatus};
use std::time::{Duration, Instant};

#[test]
fn test_rate_limit_status_expiry_with_reset() {
    let now = Instant::now();
    let reset_at = now + Duration::from_secs(30);

    let status = RateLimitStatus::new(
        ModelId::Gemini(GeminiModel::Gemini25Flash),
        now,
        Some(reset_at),
    );

    assert!(!status.is_likely_expired());
}

#[test]
fn test_rate_limit_status_expiry_default_cooldown() {
    let detected_at = Instant::now() - Duration::from_secs(61);

    let status = RateLimitStatus::new(
        ModelId::Groq(GroqModel::Llama33_70BVersatile),
        detected_at,
        None,
    );

    assert!(status.is_likely_expired());
}

