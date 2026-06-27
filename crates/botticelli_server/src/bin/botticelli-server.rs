//! botticelli-server: inference backend + bot runtime.
//!
//! Select a backend with a subcommand:
//!
//! ```text
//! botticelli-server mistral --model-path ./models/llama.gguf
//! botticelli-server ollama  --model llama3.2
//! ```

use anyhow::Context as _;
use botticelli_interface::BotticelliDriver;
use botticelli_server::BotServer;
#[cfg(feature = "mistral")]
use botticelli_server::{ServerError, ServerErrorKind};
use clap::{Parser, Subcommand};
use std::fs::OpenOptions;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

#[derive(Parser)]
#[command(
    name = "botticelli-server",
    about = "Botticelli inference server",
    version
)]
struct Cli {
    /// Hours between generation cycles (default: 24).
    #[arg(long, default_value = "24")]
    generation_hours: u64,

    /// Hours between curation cycles (default: 6).
    #[arg(long, default_value = "6")]
    curation_hours: u64,

    /// Hours between posting cycles (default: 1).
    #[arg(long, default_value = "1")]
    posting_hours: u64,

    /// Port for the metrics HTTP server (disabled if not set).
    #[arg(long)]
    metrics_port: Option<u16>,

    /// Path to the log file (appends on each run).
    #[arg(long, default_value = "botticelli-server.log")]
    log_file: String,

    #[command(subcommand)]
    backend: Backend,
}

#[derive(Subcommand)]
enum Backend {
    /// Embedded inference via mistral-rs (self-contained, no external process).
    ///
    /// Accepts a local model directory (safetensors or GGUF) or a HuggingFace repo ID.
    #[cfg(feature = "mistral")]
    Mistral {
        /// Local path to model directory or HuggingFace repo ID (e.g. "Qwen/Qwen2.5-0.5B").
        #[arg(long)]
        model_path: String,

        /// Human-readable model identifier (defaults to the last path component).
        #[arg(long)]
        model_id: Option<String>,
    },

    /// External Ollama inference server.
    #[cfg(feature = "ollama")]
    Ollama {
        /// Ollama server URL.
        #[arg(long, default_value = "http://localhost:11434")]
        url: String,

        /// Model name to use (e.g., "llama3.2").
        #[arg(long)]
        model: String,
    },

    /// Placeholder used when no backend features were compiled in.
    ///
    /// Always hidden from help; produces a clear runtime error rather than
    /// an opaque compile-time failure.
    #[cfg(not(any(feature = "mistral", feature = "ollama")))]
    #[command(hide = true)]
    NoBackend,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&cli.log_file)
        .with_context(|| format!("opening log file: {}", cli.log_file))?;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(log_file)
        .with_ansi(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(stderr_layer)
        .with(file_layer)
        .init();

    info!(
        generation_hours = cli.generation_hours,
        curation_hours = cli.curation_hours,
        posting_hours = cli.posting_hours,
        log_file = %cli.log_file,
        "botticelli-server starting"
    );

    let driver: Arc<dyn BotticelliDriver> = match cli.backend {
        #[cfg(feature = "mistral")]
        Backend::Mistral {
            ref model_path,
            ref model_id,
        } => {
            use botticelli_server::MistralConfigBuilder;

            let resolved_id = model_id.clone().unwrap_or_else(|| {
                std::path::Path::new(model_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| model_path.clone())
            });

            info!(model = %resolved_id, path = %model_path, "Loading mistral-rs model");

            let config = MistralConfigBuilder::default()
                .model_path(model_path.clone())
                .model_id(resolved_id)
                .build()
                .map_err(|e| {
                    ServerError::new(ServerErrorKind::Configuration(format!(
                        "invalid MistralConfig: {e}"
                    )))
                })
                .context("building MistralConfig")?;

            let driver = botticelli_server::MistralDriver::load(config)
                .await
                .context("loading mistral-rs model")?;
            Arc::new(driver)
        }

        #[cfg(feature = "ollama")]
        Backend::Ollama { ref url, ref model } => {
            use botticelli_models::OllamaClient;

            info!(url = %url, model = %model, "Connecting to Ollama");

            let client = OllamaClient::new_with_url(model.clone(), url.clone())?;
            Arc::new(client)
        }

        #[cfg(not(any(feature = "mistral", feature = "ollama")))]
        Backend::NoBackend => {
            anyhow::bail!(
                "botticelli-server was compiled without any inference backend.\n\
                 Rebuild with at least one backend feature:\n\
                 \n\
                 cargo build -p botticelli_server --features mistral\n\
                 cargo build -p botticelli_server --features ollama"
            );
        }
    };

    let mut server = BotServer::new(driver);

    server
        .start(
            Duration::from_secs(cli.generation_hours * 3600),
            Duration::from_secs(cli.curation_hours * 3600),
            Duration::from_secs(cli.posting_hours * 3600),
            cli.metrics_port,
        )
        .await
        .context("starting bot server")?;

    tokio::signal::ctrl_c().await?;
    info!("Received Ctrl-C, shutting down");
    server.stop().await?;

    Ok(())
}
