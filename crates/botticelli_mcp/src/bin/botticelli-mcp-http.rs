//! Botticelli MCP HTTP server binary.
//!
//! Runs the MCP server over HTTP with SSE streaming support for web access.

use anyhow::Result;
use botticelli_mcp::BotticelliServer;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, session::local::LocalSessionManager, tower::StreamableHttpService,
};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Starting Botticelli MCP HTTP server");

    // Get port from environment or use default
    let port = std::env::var("MCP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let bind_address = format!("127.0.0.1:{}", port);

    // Create cancellation token for graceful shutdown
    let ct = CancellationToken::new();

    // Server factory function
    let server_factory = || -> Result<BotticelliServer, std::io::Error> {
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
                let model = std::env::var("GROQ_MODEL")
                    .unwrap_or_else(|_| "llama-3.1-8b-instant".to_string());
                match botticelli_models::GroqDriver::new(model) {
                    Ok(client) => {
                        tracing::info!("Groq backend initialized");
                        builder = builder.groq(std::sync::Arc::new(client));
                    }
                    Err(e) => tracing::warn!("Failed to initialize Groq: {}", e),
                }
            }
        }

        Ok(builder.build())
    };

    // Create streamable HTTP service
    let service: StreamableHttpService<BotticelliServer, LocalSessionManager> =
        StreamableHttpService::new(
            server_factory,
            Default::default(),
            StreamableHttpServerConfig {
                stateful_mode: true,
                sse_keep_alive: None,
                cancellation_token: ct.child_token(),
            },
        );

    // Create axum router and mount service
    let router = axum::Router::new().nest_service("/mcp", service);

    // Bind TCP listener
    let tcp_listener = tokio::net::TcpListener::bind(&bind_address).await?;
    tracing::info!("Server listening on http://{}/mcp", bind_address);

    // Run server with graceful shutdown
    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;

    tracing::info!("Server shutdown complete");
    Ok(())
}
