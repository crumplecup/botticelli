//! Tests for provider capability queries.

use botticelli_interface::BotticelliDriver;

#[cfg(feature = "anthropic")]
use botticelli_models::AnthropicClient;
#[cfg(feature = "gemini")]
use botticelli_models::GeminiClient;
#[cfg(feature = "groq")]
use botticelli_models::GroqDriver;
#[cfg(feature = "huggingface")]
use botticelli_models::HuggingFaceDriver;
#[cfg(feature = "ollama")]
use botticelli_models::OllamaClient;

#[test]
#[cfg(feature = "anthropic")]
fn test_anthropic_capabilities() {
    let client = AnthropicClient::new("test-key", "claude-3-5-sonnet-20241022");
    let caps = client.capabilities();

    assert!(caps.streaming(), "Anthropic should support streaming");
    assert!(caps.tool_calling(), "Anthropic should support tool calling");
    assert!(caps.vision(), "Anthropic should support vision");
    assert!(!caps.audio(), "Anthropic should not support audio");
    assert!(!caps.video(), "Anthropic should not support video");
    assert!(!caps.embeddings(), "Anthropic should not support embeddings");
    assert!(caps.json_mode(), "Anthropic should support JSON mode");
    assert!(
        !caps.batch_generation(),
        "Anthropic should not support batch generation"
    );
}

#[test]
#[cfg(feature = "gemini")]
fn test_gemini_capabilities() {
    // Skip test if GEMINI_API_KEY is not set
    if std::env::var("GEMINI_API_KEY").is_err() {
        eprintln!("Skipping test_gemini_capabilities: GEMINI_API_KEY not set");
        return;
    }

    let client = GeminiClient::new().expect("Failed to create GeminiClient");
    let caps = client.capabilities();

    assert!(caps.streaming(), "Gemini should support streaming");
    assert!(caps.tool_calling(), "Gemini should support tool calling");
    assert!(caps.vision(), "Gemini should support vision");
    assert!(caps.audio(), "Gemini should support audio");
    assert!(caps.video(), "Gemini should support video");
    assert!(caps.embeddings(), "Gemini should support embeddings");
    assert!(caps.json_mode(), "Gemini should support JSON mode");
    assert!(
        !caps.batch_generation(),
        "Gemini should not support batch generation"
    );
}

#[test]
#[cfg(feature = "ollama")]
fn test_ollama_capabilities() {
    let client = OllamaClient::new("llama2").expect("Failed to create OllamaClient");
    let caps = client.capabilities();

    assert!(caps.streaming(), "Ollama should support streaming");
    assert!(!caps.tool_calling(), "Ollama should not support tool calling");
    assert!(!caps.vision(), "Ollama should not support vision");
    assert!(!caps.audio(), "Ollama should not support audio");
    assert!(!caps.video(), "Ollama should not support video");
    assert!(!caps.embeddings(), "Ollama should not support embeddings");
    assert!(!caps.json_mode(), "Ollama should not support JSON mode");
    assert!(
        !caps.batch_generation(),
        "Ollama should not support batch generation"
    );
}

#[test]
#[cfg(feature = "groq")]
fn test_groq_capabilities() {
    let client = GroqDriver::with_api_key("test-key".to_string(), "llama3-8b-8192".to_string())
        .expect("Failed to create GroqDriver");
    let caps = client.capabilities();

    assert!(!caps.streaming(), "Groq does not support real streaming");
    assert!(!caps.tool_calling(), "Groq does not support tool calling");
    assert!(!caps.vision(), "Groq should not support vision");
    assert!(!caps.audio(), "Groq should not support audio");
    assert!(!caps.video(), "Groq should not support video");
    assert!(!caps.embeddings(), "Groq should not support embeddings");
    assert!(caps.json_mode(), "Groq should support JSON mode");
    assert!(
        !caps.batch_generation(),
        "Groq should not support batch generation"
    );
}

#[test]
#[cfg(feature = "huggingface")]
fn test_huggingface_capabilities() {
    let client = HuggingFaceDriver::with_api_token(
        "test-key".to_string(),
        "meta-llama/Llama-2-7b-chat-hf".to_string(),
    )
    .expect("Failed to create HuggingFaceDriver");
    let caps = client.capabilities();

    assert!(
        !caps.streaming(),
        "HuggingFace does not support real streaming"
    );
    assert!(
        !caps.tool_calling(),
        "HuggingFace does not support tool calling"
    );
    assert!(!caps.vision(), "HuggingFace should not support vision");
    assert!(!caps.audio(), "HuggingFace should not support audio");
    assert!(!caps.video(), "HuggingFace should not support video");
    assert!(
        !caps.embeddings(),
        "HuggingFace should not support embeddings"
    );
    assert!(caps.json_mode(), "HuggingFace should support JSON mode");
    assert!(
        !caps.batch_generation(),
        "HuggingFace should not support batch generation"
    );
}
