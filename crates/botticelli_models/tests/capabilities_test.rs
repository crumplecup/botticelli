//! Tests for provider capability queries.

mod helpers;

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
    helpers::init_test_tracing("info");
    tracing::info!("Testing Anthropic capabilities");

    let client = AnthropicClient::new("test-key", "claude-3-5-sonnet-20241022");
    let caps = client.capabilities();

    tracing::debug!(
        streaming = caps.streaming(),
        tool_calling = caps.tool_calling(),
        vision = caps.vision(),
        audio = caps.audio(),
        video = caps.video(),
        embeddings = caps.embeddings(),
        json_mode = caps.json_mode(),
        batch = caps.batch_generation(),
        "Anthropic capabilities"
    );

    assert!(caps.streaming(), "Anthropic should support streaming");
    assert!(caps.tool_calling(), "Anthropic should support tool calling");
    assert!(caps.vision(), "Anthropic should support vision");
    assert!(!caps.audio(), "Anthropic should not support audio");
    assert!(!caps.video(), "Anthropic should not support video");
    assert!(
        !caps.embeddings(),
        "Anthropic should not support embeddings"
    );
    assert!(caps.json_mode(), "Anthropic should support JSON mode");
    assert!(
        !caps.batch_generation(),
        "Anthropic should not support batch generation"
    );

    tracing::info!("Anthropic capabilities test passed");
}

#[test]
#[cfg(feature = "gemini")]
fn test_gemini_capabilities() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Gemini capabilities");

    // Load .env
    let _ = dotenvy::dotenv();

    let client = GeminiClient::new()?;
    let caps = client.capabilities();

    tracing::debug!(
        streaming = caps.supports_streaming(),
        tool_calling = caps.supports_tool_calling(),
        vision = caps.supports_vision(),
        audio = caps.supports_audio(),
        video = caps.supports_video(),
        embeddings = caps.supports_embeddings(),
        json_mode = caps.supports_json_mode(),
        batch = caps.supports_batch(),
        "Gemini capabilities"
    );

    assert!(caps.supports_streaming(), "Gemini should support streaming");
    assert!(
        caps.supports_tool_calling(),
        "Gemini should support tool calling"
    );
    assert!(caps.supports_vision(), "Gemini should support vision");
    assert!(caps.supports_audio(), "Gemini should support audio");
    assert!(caps.supports_video(), "Gemini should support video");
    assert!(
        !caps.supports_embeddings(),
        "Gemini should not support embeddings"
    );
    assert!(caps.supports_json_mode(), "Gemini should support JSON mode");
    assert!(
        !caps.supports_batch(),
        "Gemini should not support batch generation"
    );

    tracing::info!("Gemini capabilities test passed");
    Ok(())
}

#[test]
#[cfg(feature = "ollama")]
fn test_ollama_capabilities() -> Result<(), botticelli_error::ModelsError> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Ollama capabilities");

    let client = OllamaClient::new("llama2")?;
    let caps = client.capabilities();

    tracing::debug!(
        streaming = caps.streaming(),
        tool_calling = caps.tool_calling(),
        vision = caps.vision(),
        audio = caps.audio(),
        video = caps.video(),
        embeddings = caps.embeddings(),
        json_mode = caps.json_mode(),
        batch = caps.batch_generation(),
        "Ollama capabilities"
    );

    assert!(caps.streaming(), "Ollama should support streaming");
    assert!(
        !caps.tool_calling(),
        "Ollama should not support tool calling"
    );
    assert!(!caps.vision(), "Ollama should not support vision");
    assert!(!caps.audio(), "Ollama should not support audio");
    assert!(!caps.video(), "Ollama should not support video");
    assert!(!caps.embeddings(), "Ollama should not support embeddings");
    assert!(!caps.json_mode(), "Ollama should not support JSON mode");
    assert!(
        !caps.batch_generation(),
        "Ollama should not support batch generation"
    );

    tracing::info!("Ollama capabilities test passed");
    Ok(())
}

#[test]
#[cfg(feature = "groq")]
fn test_groq_capabilities() -> Result<(), botticelli_error::ModelsError> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing Groq capabilities");

    let client = GroqDriver::with_api_key("test-key".to_string(), "llama3-8b-8192".to_string())?;
    let caps = client.capabilities();

    tracing::debug!(
        streaming = caps.streaming(),
        tool_calling = caps.tool_calling(),
        vision = caps.vision(),
        audio = caps.audio(),
        video = caps.video(),
        embeddings = caps.embeddings(),
        json_mode = caps.json_mode(),
        batch = caps.batch_generation(),
        "Groq capabilities"
    );

    assert!(!caps.streaming(), "Groq does not support real streaming");
    assert!(
        caps.tool_calling(),
        "Groq supports tool calling via OpenAI-compatible API"
    );
    assert!(!caps.vision(), "Groq should not support vision");
    assert!(!caps.audio(), "Groq should not support audio");
    assert!(!caps.video(), "Groq should not support video");
    assert!(!caps.embeddings(), "Groq should not support embeddings");
    assert!(caps.json_mode(), "Groq should support JSON mode");
    assert!(
        !caps.batch_generation(),
        "Groq should not support batch generation"
    );

    tracing::info!("Groq capabilities test passed");
    Ok(())
}

#[test]
#[cfg(feature = "huggingface")]
fn test_huggingface_capabilities() -> Result<(), botticelli_error::ModelsError> {
    helpers::init_test_tracing("info");
    tracing::info!("Testing HuggingFace capabilities");

    let client = HuggingFaceDriver::with_api_token(
        "test-key".to_string(),
        "meta-llama/Llama-2-7b-chat-hf".to_string(),
    )?;
    let caps = client.capabilities();

    tracing::debug!(
        streaming = caps.streaming(),
        tool_calling = caps.tool_calling(),
        vision = caps.vision(),
        audio = caps.audio(),
        video = caps.video(),
        embeddings = caps.embeddings(),
        json_mode = caps.json_mode(),
        batch = caps.batch_generation(),
        "HuggingFace capabilities"
    );

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

    tracing::info!("HuggingFace capabilities test passed");
    Ok(())
}
