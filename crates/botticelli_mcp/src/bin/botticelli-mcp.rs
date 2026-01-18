//! Botticelli MCP server binary (stdio transport).
//!
//! This server exposes Botticelli's LLM orchestration capabilities via MCP over stdin/stdout.

use anyhow::Result;
use botticelli_mcp::BotticelliServer;
use rmcp::ServiceExt;
use rmcp::transport::io::stdio;
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    let _ = dotenvy::dotenv();

    // Initialize tracing - write to STDERR to avoid corrupting JSON-RPC on stdout
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting Botticelli MCP server (stdio)");

    // Build server with available backends
    let mut builder = BotticelliServer::builder();

    #[cfg(feature = "gemini")]
    {
        if std::env::var("GEMINI_API_KEY").is_ok() {
            match botticelli_models::GeminiClient::new() {
                Ok(client) => {
                    tracing::info!("Gemini backend initialized");
                    builder = builder.gemini(std::sync::Arc::new(client));
                }
                Err(e) => tracing::warn!("Failed to initialize Gemini: {}", e),
            }
        }
    }

    #[cfg(feature = "anthropic")]
    {
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            let model = std::env::var("ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string());
            let client = botticelli_models::AnthropicClient::new(api_key, model);
            tracing::info!("Anthropic backend initialized");
            builder = builder.anthropic(std::sync::Arc::new(client));
        }
    }

    #[cfg(feature = "ollama")]
    {
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());
        match botticelli_models::OllamaClient::new(model) {
            Ok(client) => {
                tracing::info!("Ollama backend initialized");
                builder = builder.ollama(std::sync::Arc::new(client));
            }
            Err(e) => tracing::warn!("Failed to initialize Ollama: {}", e),
        }
    }

    #[cfg(feature = "huggingface")]
    {
        if std::env::var("HUGGINGFACE_API_KEY").is_ok() {
            let model = std::env::var("HUGGINGFACE_MODEL")
                .unwrap_or_else(|_| "mistralai/Mistral-7B-Instruct-v0.2".to_string());
            match botticelli_models::HuggingFaceDriver::new(model) {
                Ok(client) => {
                    tracing::info!("HuggingFace backend initialized");
                    builder = builder.huggingface(std::sync::Arc::new(client));
                }
                Err(e) => tracing::warn!("Failed to initialize HuggingFace: {}", e),
            }
        }
    }

    #[cfg(feature = "groq")]
    {
        if std::env::var("GROQ_API_KEY").is_ok() {
            let model =
                std::env::var("GROQ_MODEL").unwrap_or_else(|_| "llama-3.1-8b-instant".to_string());
            match botticelli_models::GroqDriver::new(model) {
                Ok(client) => {
                    tracing::info!("Groq backend initialized");
                    builder = builder.groq(std::sync::Arc::new(client));
                }
                Err(e) => tracing::warn!("Failed to initialize Groq: {}", e),
            }
        }
    }

    let server = builder.build();

    tracing::info!("Server ready, listening on stdio");

    // Run server with stdio transport
    let _running_service = server.serve(stdio()).await?;

    // Server runs forever until process is killed
    std::future::pending::<()>().await;

    Ok(())
}
