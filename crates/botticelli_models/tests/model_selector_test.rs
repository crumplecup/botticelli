use botticelli_models::{GeminiModel, ModelBounds, ModelFamily, ModelId};

#[test]
fn test_bounds_none_allows_all() {
    let bounds = ModelBounds::none();
    assert!(bounds.allows(ModelId::Gemini(GeminiModel::Gemini25Pro)));
    assert!(bounds.allows(ModelId::Gemini(GeminiModel::Gemini25FlashLite)));
}

#[test]
fn test_bounds_no_lower_than() {
    let bounds = ModelBounds::lower_bound(ModelId::Gemini(GeminiModel::Gemini25Flash));
    assert!(bounds.allows(ModelId::Gemini(GeminiModel::Gemini25Pro)));
    assert!(bounds.allows(ModelId::Gemini(GeminiModel::Gemini25Flash)));
    assert!(!bounds.allows(ModelId::Gemini(GeminiModel::Gemini25FlashLite)));
}

#[test]
fn test_bounds_no_higher_than() {
    let bounds = ModelBounds::upper_bound(ModelId::Gemini(GeminiModel::Gemini25Flash));
    assert!(!bounds.allows(ModelId::Gemini(GeminiModel::Gemini25Pro)));
    assert!(bounds.allows(ModelId::Gemini(GeminiModel::Gemini25Flash)));
    assert!(bounds.allows(ModelId::Gemini(GeminiModel::Gemini25FlashLite)));
}

#[test]
fn test_bounds_both() {
    let bounds = ModelBounds::new(
        Some(ModelId::Gemini(GeminiModel::Gemini20Flash)),
        Some(ModelId::Gemini(GeminiModel::Gemini25Flash)),
    );
    assert!(!bounds.allows(ModelId::Gemini(GeminiModel::Gemini25Pro)));
    assert!(bounds.allows(ModelId::Gemini(GeminiModel::Gemini25Flash)));
    assert!(bounds.allows(ModelId::Gemini(GeminiModel::Gemini20Flash)));
    assert!(!bounds.allows(ModelId::Gemini(GeminiModel::Gemini25FlashLite)));
}

#[test]
fn test_model_id_family() {
    assert_eq!(
        ModelId::Gemini(GeminiModel::Gemini25Flash).family(),
        ModelFamily::Gemini
    );
    assert_eq!(
        ModelId::Groq(botticelli_models::GroqModel::Llama31_8BInstant).family(),
        ModelFamily::Groq
    );
}

#[test]
fn test_model_id_move_up() {
    let model = ModelId::Gemini(GeminiModel::Gemini25Flash);
    assert_eq!(
        model.move_up(),
        Some(ModelId::Gemini(GeminiModel::Gemini25Pro))
    );
}

#[test]
fn test_model_id_move_down() {
    let model = ModelId::Gemini(GeminiModel::Gemini25Flash);
    assert_eq!(
        model.move_down(),
        Some(ModelId::Gemini(GeminiModel::Gemini20FlashThinking))
    );
}

#[test]
fn test_model_id_friends() {
    let model = ModelId::Gemini(GeminiModel::Gemini25Flash);
    let friends = model.friends();
    assert!(!friends.is_empty());
    assert!(friends.iter().all(|f| f.family() != ModelFamily::Gemini));
}

#[test]
fn test_is_at_least_same_family() {
    let pro = ModelId::Gemini(GeminiModel::Gemini25Pro);
    let flash = ModelId::Gemini(GeminiModel::Gemini25Flash);
    let lite = ModelId::Gemini(GeminiModel::Gemini25FlashLite);

    assert!(pro.is_at_least(pro));
    assert!(pro.is_at_least(flash));
    assert!(pro.is_at_least(lite));
    assert!(!flash.is_at_least(pro));
    assert!(flash.is_at_least(flash));
    assert!(flash.is_at_least(lite));
}

#[test]
fn test_is_at_most_same_family() {
    let pro = ModelId::Gemini(GeminiModel::Gemini25Pro);
    let flash = ModelId::Gemini(GeminiModel::Gemini25Flash);
    let lite = ModelId::Gemini(GeminiModel::Gemini25FlashLite);

    assert!(lite.is_at_most(pro));
    assert!(lite.is_at_most(flash));
    assert!(lite.is_at_most(lite));
    assert!(!pro.is_at_most(flash));
    assert!(flash.is_at_most(flash));
    assert!(flash.is_at_most(pro));
}
