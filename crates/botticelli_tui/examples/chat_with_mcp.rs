//! Example: Chat with MCP tool integration
//!
//! This example demonstrates running the Botticelli TUI with full MCP integration,
//! allowing the LLM to use narrative tools during conversation.
//!
//! # Setup
//!
//! 1. Set your Anthropic API key:
//!    ```bash
//!    export ANTHROPIC_API_KEY=sk-ant-...
//!    ```
//!
//! 2. Create narratives directory:
//!    ```bash
//!    mkdir -p narratives
//!    ```
//!
//! 3. Run the example:
//!    ```bash
//!    cargo run --example chat_with_mcp
//!    ```
//!
//! # Usage
//!
//! - Type messages in the input box at the bottom
//! - Press Enter to send
//! - The LLM can use tools like:
//!   - `echo`: Simple echo for testing
//!   - `create_narrative`: Create narratives from TOML
//!   - `validate_narrative`: Validate narrative structure
//!   - `list_narratives`: List available narratives
//!   - `load_narrative`: Load narrative from file
//!
//! - Press Ctrl+C or 'q' to quit
//!
//! # Example Prompts
//!
//! - "Echo hello world" - Tests the echo tool
//! - "List available narratives" - Lists narratives in ./narratives/
//! - "Create a simple narrative with 3 acts" - Tests narrative creation
//!

use botticelli_models::AnthropicClient;
use botticelli_tui::TuiApp;
use std::sync::Arc;
use tracing::error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("botticelli_tui=debug".parse()?)
                .add_directive("botticelli_mcp_client=debug".parse()?),
        )
        .init();

    // Get API key from environment
    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
        "ANTHROPIC_API_KEY environment variable not set. Set it with: export ANTHROPIC_API_KEY=sk-ant-..."
    })?;

    // Create Anthropic driver
    let driver = Arc::new(AnthropicClient::new(api_key, "claude-3-5-sonnet-20241022"));

    // Create and run TUI application
    let mut app = TuiApp::new(driver)?;

    if let Err(e) = app.run().await {
        error!("TUI error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
