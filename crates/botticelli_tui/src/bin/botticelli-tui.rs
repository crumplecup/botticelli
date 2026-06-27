#![recursion_limit = "256"]
//! botticelli-tui: operator console for the Botticelli bot server.

use std::sync::Arc;

use async_trait::async_trait;
use botticelli_error::BuilderError;
use botticelli_interface::BotticelliDriver;
use botticelli_rate_limit::RateLimitConfig;
use botticelli_tui::{BotController, BotScreenContext, TuiResult};
use clap::Parser;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::oneshot;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Which LLM backend to use for chat.
#[derive(Debug, Clone, clap::ValueEnum)]
enum Provider {
    /// Running botticelli-server over MCP — started in-process automatically (default)
    Server,
    /// Google Gemini — reads GEMINI_API_KEY from environment
    Gemini,
    /// Anthropic Claude — reads ANTHROPIC_API_KEY from environment
    Anthropic,
    /// Local Ollama — no API key needed
    Ollama,
}

/// Botticelli TUI operator console.
#[derive(Parser, Debug, Clone)]
#[command(version, about)]
struct Args {
    /// LLM backend to use for the Chat screen.
    #[arg(long, value_enum, default_value = "server")]
    provider: Provider,

    /// Model name forwarded to the backend.
    ///
    /// Defaults vary by provider:
    ///   server    → "Qwen/Qwen2.5-Coder-0.5B-Instruct" (local HuggingFace)
    ///   gemini    → "gemini-2.0-flash-exp"
    ///   anthropic → "claude-sonnet-4-6"
    ///   ollama    → "mistral"
    #[arg(long)]
    model: Option<String>,

    /// Connect to an external botticelli-server at this URL instead of starting one in-process.
    ///
    /// Only used with `--provider server`. When omitted, the TUI starts the server
    /// automatically in the same process — no manual `botticelli-mcp http` needed.
    #[arg(long)]
    server_url: Option<String>,
}

#[tokio::main]
async fn main() -> TuiResult<()> {
    dotenvy::dotenv().ok();
    let args = Args::parse();

    // Log to file only — never stderr, which bleeds into the ratatui alternate screen.
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("botticelli-tui.log")?;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(log_file)
                .with_ansi(false),
        )
        .init();

    info!("botticelli-tui starting");

    // Spawn model load in the background so the TUI opens immediately.
    let (driver_tx, driver_rx) = oneshot::channel::<TuiResult<Arc<dyn BotticelliDriver>>>();
    let args_for_load = args.clone();
    tokio::spawn(async move {
        driver_tx.send(build_driver(&args_for_load).await).ok();
    });

    // Setup terminal immediately — don't wait for the model.
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let storage = botticelli_database::open_storage_from_env().await.ok();
    let narratives_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("narratives");
    let ctx = BotScreenContext {
        narratives_dir: Some(narratives_dir),
        log_file: Some(std::path::PathBuf::from("botticelli-server.log")),
        storage,
    };

    let mut controller = BotController::new(ctx).with_driver_receiver(driver_rx);
    let result = controller.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn build_driver(args: &Args) -> TuiResult<Arc<dyn BotticelliDriver>> {
    match args.provider {
        Provider::Server => {
            let model = args
                .model
                .clone()
                .unwrap_or_else(|| "Qwen/Qwen2.5-Coder-0.5B-Instruct".to_string());
            info!(model, "Loading local inference model via mistral-rs");
            let config = botticelli_server::MistralConfigBuilder::default()
                .model_path(model.clone())
                .model_id(model)
                .build()
                .map_err(|e| BuilderError::from(e.to_string()))?;
            let driver = botticelli_server::MistralDriver::load(config).await?;
            Ok(Arc::new(driver))
        }
        Provider::Gemini => {
            let model = args
                .model
                .clone()
                .unwrap_or_else(|| "gemini-2.0-flash-exp".to_string());
            let client = botticelli_models::GeminiClient::new()?;
            Ok(Arc::new(GeminiDriverWithModel::new(client, model)))
        }
        Provider::Anthropic => {
            let model = args
                .model
                .clone()
                .unwrap_or_else(|| "claude-sonnet-4-6".to_string());
            let key = std::env::var("ANTHROPIC_API_KEY")?;
            Ok(Arc::new(botticelli_models::AnthropicClient::new(
                key, model,
            )))
        }
        Provider::Ollama => {
            let model = args.model.clone().unwrap_or_else(|| "mistral".to_string());
            let client = botticelli_models::OllamaClient::new(&model)
                .map_err(botticelli_error::BotticelliError::from)?;
            Ok(Arc::new(client))
        }
    }
}

/// Wraps [`botticelli_models::GeminiClient`] to inject a default model name
/// into requests that don't specify one.
struct GeminiDriverWithModel {
    inner: botticelli_models::GeminiClient,
    model: String,
    rate_limits: RateLimitConfig,
}

impl GeminiDriverWithModel {
    fn new(inner: botticelli_models::GeminiClient, model: String) -> Self {
        Self {
            inner,
            model,
            rate_limits: RateLimitConfig::unlimited("gemini-tui"),
        }
    }
}

#[async_trait]
impl BotticelliDriver for GeminiDriverWithModel {
    async fn generate(
        &self,
        req: &botticelli_core::GenerateRequest,
    ) -> botticelli_error::BotticelliResult<botticelli_core::GenerateResponse> {
        if req.model().is_none() {
            let req2 = req.clone().with_model(Some(self.model.clone()));
            self.inner.generate(&req2).await
        } else {
            self.inner.generate(req).await
        }
    }

    fn provider_name(&self) -> &'static str {
        "gemini"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn rate_limits(&self) -> &RateLimitConfig {
        &self.rate_limits
    }
}
