use botticelli_models::{claude_tokenizer, count_tokens_tiktoken, gpt_tokenizer};

#[test]
fn test_claude_tokenizer() {
    let tokenizer = claude_tokenizer().expect("Failed to load tokenizer");
    let count = count_tokens_tiktoken("Hello, world!", &tokenizer);
    assert!(count > 0);
    assert!(count < 10);
}

#[test]
fn test_gpt_tokenizer() {
    let tokenizer = gpt_tokenizer().expect("Failed to load tokenizer");
    let count = count_tokens_tiktoken("Hello, world!", &tokenizer);
    assert!(count > 0);
    assert!(count < 10);
}
