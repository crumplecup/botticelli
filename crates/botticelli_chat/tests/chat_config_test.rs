//! Tests for ChatConfig.

use botticelli_chat::ChatConfig;
use botticelli_models::{GeminiModel, GroqModel, ModelBounds, ModelId, SelectionStrategy};

#[test]
fn test_default_config() {
    let config = ChatConfig::default();
    assert!(config.model_bounds().is_none());
    assert_eq!(
        config.initial_model(),
        &ModelId::Gemini(GeminiModel::Gemini25Flash)
    );
}

#[test]
fn test_config_with_bounds() {
    let bounds = ModelBounds::lower_bound(ModelId::Gemini(GeminiModel::Gemini25Flash));
    let config = ChatConfig::new(
        Some(bounds),
        ModelId::Gemini(GeminiModel::Gemini25Flash),
        SelectionStrategy::FriendlyFirst,
    );

    assert!(config.is_within_bounds(&ModelId::Gemini(GeminiModel::Gemini25Flash)));
    assert!(config.is_within_bounds(&ModelId::Gemini(GeminiModel::Gemini25Pro)));
    assert!(!config.is_within_bounds(&ModelId::Gemini(GeminiModel::Gemini25FlashLite)));
}

#[test]
fn test_config_without_bounds() {
    let config = ChatConfig::new(
        None,
        ModelId::Groq(GroqModel::Llama33_70BVersatile),
        SelectionStrategy::LoyalFirst,
    );

    assert!(config.is_within_bounds(&ModelId::Gemini(GeminiModel::Gemini25Pro)));
    assert!(config.is_within_bounds(&ModelId::Groq(GroqModel::Llama31_8BInstant)));
}

#[test]
fn test_serialize_deserialize() {
    let bounds = ModelBounds::both(
        ModelId::Gemini(GeminiModel::Gemini25FlashLite),
        ModelId::Gemini(GeminiModel::Gemini25Pro),
    );
    let config = ChatConfig::new(
        Some(bounds),
        ModelId::Gemini(GeminiModel::Gemini25Flash),
        SelectionStrategy::FriendlyFirst,
    );

    let toml = toml::to_string(&config).expect("Serialize failed");
    let deserialized: ChatConfig = toml::from_str(&toml).expect("Deserialize failed");

    assert_eq!(config.initial_model(), deserialized.initial_model());
    assert!(deserialized.model_bounds().is_some());
}
