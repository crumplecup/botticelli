//! Botticelli MCP server binary.
//!
//! Unified server that supports both stdio and HTTP transports.

use anyhow::Result;
use botticelli_mcp::BotticelliServer;
use clap::{Parser, ValueEnum};
use rmcp::ServiceExt;
use tracing_subscriber::{self, EnvFilter};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Transport {
    /// Standard input/output transport (for desktop integration)
    Stdio,
    /// HTTP transport with SSE streaming (for web access)
    Http,
}

#[derive(Parser)]
#[command(name = "botticelli_mcp")]
#[command(about = "Botticelli Model Context Protocol Server", long_about = None)]
struct Cli {
    /// Transport mode to use
    #[arg(value_enum, default_value_t = Transport::Stdio)]
    transport: Transport,

    /// Port for HTTP server (only used with http transport)
    #[arg(short, long, default_value_t = 3000)]
    port: u16,
}

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    // Initialize tracing based on transport
    match cli.transport {
        Transport::Stdio => {
            // Write to STDERR to avoid corrupting JSON-RPC on stdout
            tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
                )
                .with_target(false)
                .with_thread_ids(false)
                .with_file(true)
                .with_line_number(true)
                .with_writer(std::io::stderr)
                .init();
            tracing::info!("Starting Botticelli MCP server (stdio)");
        }
        Transport::Http => {
            tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
                )
                .with_target(false)
                .with_thread_ids(false)
                .with_file(true)
                .with_line_number(true)
                .init();
            tracing::info!("Starting Botticelli MCP server (http)");
        }
    }

    // Create server builder and initialize backends
    let server = initialize_backends();

    // Run with selected transport
    match cli.transport {
        Transport::Stdio => run_stdio(server).await,
        Transport::Http => run_http(server, cli.port).await,
    }
}

/// Initialize all available LLM backends from environment.
#[tracing::instrument(skip_all)]
fn initialize_backends() -> BotticelliServer {
    let mut builder = BotticelliServer::builder();

    #[cfg(feature = "gemini")]
    {
        if std::env::var("GEMINI_API_KEY").is_ok() {
            match botticelli_models::GeminiClient::new() {
                Ok(client) => {
                    tracing::info!("Gemini backend initialized");
                    builder = builder.gemini_driver(Some(std::sync::Arc::new(client)));
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
            builder = builder.anthropic_driver(Some(std::sync::Arc::new(client)));
        }
    }

    #[cfg(feature = "ollama")]
    {
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());
        match botticelli_models::OllamaClient::new(model) {
            Ok(client) => {
                tracing::info!("Ollama backend initialized");
                builder = builder.ollama_driver(Some(std::sync::Arc::new(client)));
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
                    builder = builder.huggingface_driver(Some(std::sync::Arc::new(client)));
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
                    builder = builder.groq_driver(Some(std::sync::Arc::new(client)));
                }
                Err(e) => tracing::warn!("Failed to initialize Groq: {}", e),
            }
        }
    }

    builder.build().expect("Failed to build server")
}

/// Run server with stdio transport.
#[tracing::instrument(skip_all)]
async fn run_stdio(server: BotticelliServer) -> Result<()> {
    use rmcp::transport::io::stdio;

    tracing::info!("Server ready, listening on stdio");

    // Run server with stdio transport
    let _running_service = server.serve(stdio()).await?;

    // Server runs forever until process is killed
    std::future::pending::<()>().await;

    Ok(())
}

/// Run server with HTTP transport.
#[cfg(feature = "http")]
#[tracing::instrument(skip_all, fields(port))]
async fn run_http(_server: BotticelliServer, port: u16) -> Result<()> {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, session::local::LocalSessionManager,
        tower::StreamableHttpService,
    };
    use tokio_util::sync::CancellationToken;

    let bind_address = format!("127.0.0.1:{}", port);
    tracing::Span::current().record("port", port);

    // Create cancellation token for graceful shutdown
    let ct = CancellationToken::new();

    // Server factory - creates a new server instance for each session
    let server_factory =
        move || -> Result<BotticelliServer, std::io::Error> { Ok(initialize_backends()) };

    // Create streamable HTTP service
    let service: StreamableHttpService<BotticelliServer, LocalSessionManager> =
        StreamableHttpService::new(
            server_factory,
            Default::default(),
            StreamableHttpServerConfig {
                stateful_mode: true,
                sse_keep_alive: None,
                sse_retry: Some(std::time::Duration::from_secs(3)),
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

/// Fallback when HTTP feature is not enabled.
#[cfg(not(feature = "http"))]
#[tracing::instrument(skip_all)]
async fn run_http(_server: BotticelliServer, _port: u16) -> Result<()> {
    anyhow::bail!("HTTP transport requires 'http' feature to be enabled")
}
