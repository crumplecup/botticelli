use botticelli_models::ModelFamily;
use strum::IntoEnumIterator;

#[test]
fn test_fallback_order_excludes_self() {
    for family in ModelFamily::iter() {
        let fallbacks = family.fallback_order();
        assert!(
            !fallbacks.contains(&family),
            "Fallback should not include self"
        );
    }
}

#[test]
fn test_fallback_order_starts_after_current() {
    let order = ModelFamily::Gemini.fallback_order();
    assert_eq!(order[0], ModelFamily::Groq);

    let order = ModelFamily::Groq.fallback_order();
    assert_eq!(order[0], ModelFamily::Perplexity);
}

#[test]
fn test_fallback_order_wraps() {
    let order = ModelFamily::Ollama.fallback_order();
    assert_eq!(order[0], ModelFamily::Gemini);
}
