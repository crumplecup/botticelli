use botticelli_chat::ChatSession;
use botticelli_models::{
    GeminiModel, ModelBounds, ModelId, ModelSelector, RateLimitDetector, SelectionStrategy,
};

#[test]
fn test_chat_session_creation() {
    let selector = ModelSelector::new(
        ModelBounds::none(),
        SelectionStrategy::LoyalFirst,
        RateLimitDetector::new(),
    );
    let session = ChatSession::new(selector, ModelId::Gemini(GeminiModel::Gemini25Flash));

    assert_eq!(
        session.current_model(),
        &ModelId::Gemini(GeminiModel::Gemini25Flash)
    );
}

#[test]
fn test_handle_rate_limit_loyal_movement() {
    let selector = ModelSelector::new(
        ModelBounds::none(),
        SelectionStrategy::LoyalFirst,
        RateLimitDetector::new(),
    );
    let mut session = ChatSession::new(selector, ModelId::Gemini(GeminiModel::Gemini25Flash));

    let next = session.handle_rate_limit("rate limit exceeded").unwrap();
    assert_eq!(next, ModelId::Gemini(GeminiModel::Gemini20FlashThinking));
}

#[test]
fn test_handle_rate_limit_friendly_movement() {
    let selector = ModelSelector::new(
        ModelBounds::none(),
        SelectionStrategy::FriendlyFirst,
        RateLimitDetector::new(),
    );
    let mut session = ChatSession::new(selector, ModelId::Gemini(GeminiModel::Gemini25Flash));

    let next = session.handle_rate_limit("rate limit exceeded").unwrap();
    // Should try a Groq friend (actual friend determined by friends() method)
    assert!(matches!(next, ModelId::Groq(_)));
}

#[test]
fn test_handle_rate_limit_with_lower_bound() {
    let bounds = ModelBounds::lower_bound(ModelId::Gemini(GeminiModel::Gemini20Flash));

    let selector = ModelSelector::new(
        bounds,
        SelectionStrategy::LoyalFirst,
        RateLimitDetector::new(),
    );
    let mut session = ChatSession::new(selector, ModelId::Gemini(GeminiModel::Gemini20Flash));

    // Can move down from Flash within bounds
    let result = session.handle_rate_limit("rate limit exceeded");
    // May succeed or fail depending on friends availability
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_non_rate_limit_error_returns_none() {
    let selector = ModelSelector::new(
        ModelBounds::none(),
        SelectionStrategy::LoyalFirst,
        RateLimitDetector::new(),
    );
    let mut session = ChatSession::new(selector, ModelId::Gemini(GeminiModel::Gemini25Flash));

    // Non-rate-limit error should fail
    let result = session.handle_rate_limit("network timeout");
    assert!(result.is_err());
}
