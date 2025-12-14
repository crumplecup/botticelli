use botticelli_models::{
    GeminiModel, ModelBounds, ModelId, ModelSelector, RateLimitDetector, SelectionStrategy,
};

#[test]
fn test_loyal_first_moves_down_in_family() {
    let bounds = ModelBounds::none();
    let mut selector = ModelSelector::new(
        bounds,
        SelectionStrategy::LoyalFirst,
        RateLimitDetector::new(),
    );

    let current = ModelId::Gemini(GeminiModel::Gemini25Flash);
    let next = selector.select_next(current, "rate limit exceeded");

    assert_eq!(
        next,
        Some(ModelId::Gemini(GeminiModel::Gemini20FlashThinking))
    );
}

#[test]
fn test_loyal_first_tries_friendly_when_no_lower() {
    let bounds = ModelBounds::lower_bound(ModelId::Gemini(GeminiModel::Gemini25FlashLite));

    let mut selector = ModelSelector::new(
        bounds,
        SelectionStrategy::LoyalFirst,
        RateLimitDetector::new(),
    );

    let current = ModelId::Gemini(GeminiModel::Gemini25FlashLite);
    let next = selector.select_next(current, "rate limit exceeded");

    assert!(next.is_some());
    assert_ne!(next, Some(current));
    match next {
        Some(ModelId::Groq(_)) => {}
        _ => panic!("Expected Groq friend"),
    }
}

#[test]
fn test_friendly_first_tries_lateral_move() {
    let bounds = ModelBounds::none();
    let mut selector = ModelSelector::new(
        bounds,
        SelectionStrategy::FriendlyFirst,
        RateLimitDetector::new(),
    );

    let current = ModelId::Gemini(GeminiModel::Gemini25Flash);
    let next = selector.select_next(current, "rate limit exceeded");

    assert!(next.is_some());
    match next {
        Some(ModelId::Groq(_)) => {}
        _ => panic!("Expected Groq friend for friendly-first"),
    }
}

#[test]
fn test_respects_upper_bound() {
    let bounds = ModelBounds::upper_bound(ModelId::Gemini(GeminiModel::Gemini25Flash));

    let mut selector = ModelSelector::new(
        bounds,
        SelectionStrategy::LoyalFirst,
        RateLimitDetector::new(),
    );

    let current = ModelId::Gemini(GeminiModel::Gemini25Pro);
    let next = selector.select_next(current, "rate limit exceeded");

    match next {
        Some(ModelId::Gemini(GeminiModel::Gemini25Pro)) => {
            panic!("Should not stay at Pro (above bound)")
        }
        Some(id) => {
            assert!(bounds.allows(id));
        }
        None => {}
    }
}

#[test]
fn test_respects_lower_bound() {
    let bounds = ModelBounds::lower_bound(ModelId::Gemini(GeminiModel::Gemini25Flash));

    let mut selector = ModelSelector::new(
        bounds,
        SelectionStrategy::LoyalFirst,
        RateLimitDetector::new(),
    );

    let current = ModelId::Gemini(GeminiModel::Gemini25Flash);
    let next = selector.select_next(current, "rate limit exceeded");

    if let Some(id) = next {
        assert!(bounds.allows(id));
        match id {
            ModelId::Gemini(GeminiModel::Gemini25FlashLite)
            | ModelId::Gemini(GeminiModel::Gemini20Flash) => {
                panic!("Should not move below bound")
            }
            _ => {}
        }
    }
}

#[test]
fn test_non_rate_limit_error_returns_none() {
    let bounds = ModelBounds::none();
    let mut selector = ModelSelector::new(
        bounds,
        SelectionStrategy::LoyalFirst,
        RateLimitDetector::new(),
    );

    let current = ModelId::Gemini(GeminiModel::Gemini25Flash);
    let next = selector.select_next(current, "internal server error");

    assert_eq!(next, None);
}

#[test]
fn test_select_next_tracks_rate_limit() {
    let bounds = ModelBounds::none();
    let mut selector = ModelSelector::new(
        bounds,
        SelectionStrategy::FriendlyFirst,
        RateLimitDetector::new(),
    );

    let current = ModelId::Gemini(GeminiModel::Gemini25Flash);
    selector.select_next(current, "rate limit exceeded");

    let status = selector.get_status(current.family());
    assert!(status.is_some());
}
