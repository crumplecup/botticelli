use botticelli_core::{TokenUsageData, get_tokenizer};

#[test]
fn test_get_tokenizer() {
    let encoder = get_tokenizer("gpt-4").expect("Should get encoder");
    let tokens = encoder.encode_with_special_tokens("Hello, world!");
    assert!(!tokens.is_empty());
}

#[test]
fn test_token_usage_data_calculate_cost() {
    let usage = TokenUsageData::new(1_000_000, 500_000, 1_500_000);
    // $1 per million prompt, $2 per million completion
    let cost = usage.calculate_cost(1.0, 2.0);
    assert!((cost - 2.0).abs() < 0.001); // 1.0 + 1.0 = 2.0
}

#[test]
fn test_token_usage_data_default() {
    let usage = TokenUsageData::default();
    assert_eq!(*usage.input_tokens(), 0);
    assert_eq!(*usage.output_tokens(), 0);
    assert_eq!(*usage.total_tokens(), 0);
}
