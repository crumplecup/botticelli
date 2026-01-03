use botticelli_models::GeminiModel;
use strum::IntoEnumIterator;

#[test]
fn test_move_up_from_middle() {
    assert_eq!(
        GeminiModel::Gemini25Flash.move_up(),
        Some(GeminiModel::Gemini25Pro)
    );
}

#[test]
fn test_move_up_from_top() {
    assert_eq!(GeminiModel::Gemini25Pro.move_up(), None);
}

#[test]
fn test_move_down_from_middle() {
    assert_eq!(
        GeminiModel::Gemini25Flash.move_down(),
        Some(GeminiModel::Gemini20FlashThinking)
    );
}

#[test]
fn test_move_down_from_bottom() {
    assert_eq!(GeminiModel::Gemini25FlashLite.move_down(), None);
}

#[test]
fn test_friends_returns_tuples() {
    let friends = GeminiModel::Gemini25Flash.friends();
    assert!(!friends.is_empty());
    for (family, model) in friends {
        assert!(!family.is_empty());
        assert!(!model.is_empty());
    }
}

#[test]
fn test_as_str_matches_display() {
    for model in GeminiModel::iter() {
        assert_eq!(model.as_str(), model.to_string());
    }
}
