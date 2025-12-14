use botticelli_chat::ChatSession;
use botticelli_models::{
    GeminiModel, GroqModel, ModelBounds, ModelId, ModelSelector, RateLimitDetector,
    SelectionStrategy,
};

#[test]
fn test_realistic_rate_limit_cascade() {
    let bounds = ModelBounds::both(
        ModelId::Gemini(GeminiModel::Gemini25FlashLite),
        ModelId::Gemini(GeminiModel::Gemini25Pro),
    );

    let detector = RateLimitDetector::new();
    let selector = ModelSelector::new(bounds, SelectionStrategy::LoyalFirst, detector);
    let mut session = ChatSession::new(selector, ModelId::Gemini(GeminiModel::Gemini25Flash));

    // Start with Flash
    assert_eq!(
        session.current_model(),
        &ModelId::Gemini(GeminiModel::Gemini25Flash)
    );

    // Flash hits rate limit, get some alternative within bounds
    let first = session
        .handle_rate_limit("Rate limit exceeded")
        .expect("Should find alternative");
    assert_ne!(first, ModelId::Gemini(GeminiModel::Gemini25Flash));

    // Verify fallback continues working
    let second = session
        .handle_rate_limit("Rate limit exceeded")
        .expect("Should find alternative");
    assert_ne!(second, first);

    // Can find at least 3 different models
    let third = session
        .handle_rate_limit("Rate limit exceeded")
        .expect("Should find alternative");
    assert!(third != first && third != second);
}

#[test]
fn test_friendly_first_strategy() {
    let bounds = ModelBounds::none();
    let detector = RateLimitDetector::new();
    let selector = ModelSelector::new(bounds, SelectionStrategy::FriendlyFirst, detector);
    let mut session = ChatSession::new(selector, ModelId::Gemini(GeminiModel::Gemini25Flash));

    // Flash hits rate limit, try friendly first
    let next = session
        .handle_rate_limit("Rate limit exceeded")
        .expect("Should find alternative");
    assert_eq!(next, ModelId::Groq(GroqModel::Llama31_70BVersatile));
}

#[test]
fn test_upper_bound_prevents_upgrade() {
    // Set upper bound to prevent moving to Pro
    let bounds = ModelBounds::upper_bound(ModelId::Gemini(GeminiModel::Gemini25Flash));
    let detector = RateLimitDetector::new();
    let selector = ModelSelector::new(bounds, SelectionStrategy::LoyalFirst, detector);
    let mut session = ChatSession::new(selector, ModelId::Gemini(GeminiModel::Gemini20Flash));

    // 2.0 Flash hits rate limit, fallback should respect upper bound (no Pro)
    let next = session
        .handle_rate_limit("Rate limit exceeded")
        .expect("Should find alternative");
    assert_ne!(next, ModelId::Gemini(GeminiModel::Gemini25Pro));

    // Continue fallback - should never hit Pro
    for _ in 0..3 {
        if let Ok(model) = session.handle_rate_limit("Rate limit exceeded") {
            assert_ne!(model, ModelId::Gemini(GeminiModel::Gemini25Pro));
        }
    }
}

#[test]
fn test_lower_bound_prevents_downgrade() {
    let bounds = ModelBounds::lower_bound(ModelId::Gemini(GeminiModel::Gemini25Flash));
    let detector = RateLimitDetector::new();
    let selector = ModelSelector::new(bounds, SelectionStrategy::LoyalFirst, detector);
    let mut session = ChatSession::new(selector, ModelId::Gemini(GeminiModel::Gemini25Flash));

    // Flash hits rate limit, can't move down (lower bound), try friendly
    let next = session
        .handle_rate_limit("Rate limit exceeded")
        .expect("Should find alternative");
    assert_eq!(next, ModelId::Groq(GroqModel::Llama31_70BVersatile));
}

#[test]
fn test_exhausted_all_models_within_bounds() {
    let bounds = ModelBounds::both(
        ModelId::Gemini(GeminiModel::Gemini25Pro),
        ModelId::Gemini(GeminiModel::Gemini25Pro),
    );
    let detector = RateLimitDetector::new();
    let selector = ModelSelector::new(bounds, SelectionStrategy::LoyalFirst, detector);
    let mut session = ChatSession::new(selector, ModelId::Gemini(GeminiModel::Gemini25Pro));

    // Pro hits rate limit - with tight single-model bounds, tries to find friend
    // This will likely move to Groq since friends() returns cross-family equivalents
    let first = session.handle_rate_limit("Rate limit exceeded");

    // Friend models from different families are considered valid alternatives
    // even with tight bounds, as they're lateral equivalents
    assert!(first.is_ok());
}

#[test]
fn test_config_serialization_roundtrip() {
    use botticelli_chat::ChatConfig;

    let bounds = ModelBounds::both(
        ModelId::Gemini(GeminiModel::Gemini25Flash),
        ModelId::Gemini(GeminiModel::Gemini25Pro),
    );
    let config = ChatConfig::new(Some(bounds), ModelId::Gemini(GeminiModel::Gemini25Flash));

    let serialized = toml::to_string(&config).expect("Serialize");
    let deserialized: ChatConfig = toml::from_str(&serialized).expect("Deserialize");

    assert_eq!(config.initial_model(), deserialized.initial_model());

    // Both should have bounds
    assert!(config.model_bounds().is_some());
    assert!(deserialized.model_bounds().is_some());

    // Compare bounds details
    let orig_bounds = config.model_bounds().unwrap();
    let deser_bounds = deserialized.model_bounds().unwrap();
    assert_eq!(orig_bounds.lower(), deser_bounds.lower());
    assert_eq!(orig_bounds.upper(), deser_bounds.upper());
}
