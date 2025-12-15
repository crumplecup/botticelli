//! Botticelli TUI - Terminal interface with MCP integration
//!
//! # Setup
//!
//! Set your Anthropic API key:
//! ```bash
//! export ANTHROPIC_API_KEY=sk-ant-...
//! ```
//!
//! # Usage
//!
//! Run from workspace root:
//! ```bash
//! cargo run --bin tui
//! ```
//!
//! Or with logging:
//! ```bash
//! RUST_LOG=botticelli_tui=debug,botticelli_mcp_client=debug cargo run --bin tui
//! ```
//!
//! # Controls
//!
//! - Type in the input box and press Enter to send messages
//! - The LLM can use narrative tools (create, validate, list, load narratives)
//! - Press Ctrl+C or 'q' to quit
//!

use botticelli_models::AnthropicClient;
use botticelli_tui::Tui;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "botticelli_tui=info,botticelli_mcp_client=info".into()),
        )
        .init();

    // Get API key from environment
    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Error: ANTHROPIC_API_KEY environment variable not set");
            eprintln!("");
            eprintln!("Please set your Anthropic API key:");
            eprintln!("  export ANTHROPIC_API_KEY=sk-ant-...");
            eprintln!("");
            std::process::exit(1);
        }
    };

    // Create narratives directory if it doesn't exist
    if !std::path::Path::new("narratives").exists() {
        std::fs::create_dir("narratives")?;
        eprintln!("Created narratives/ directory");
    }

    // Create Anthropic driver
    let driver = Arc::new(AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022"));

    // Create TUI with MCP integration
    let mut tui = Tui::with_mcp(driver)?;

    // Run TUI
    tui.run().await?;

    Ok(())
}
