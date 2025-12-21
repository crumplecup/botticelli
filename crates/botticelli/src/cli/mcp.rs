//! MCP client command handler.

use crate::cli::McpCommandArgs;
use botticelli_mcp_client::McpHost;
use tracing::{info, instrument};

/// Handle MCP client command
#[instrument(skip_all, fields(backend = %args.backend, max_turns = args.max_turns))]
pub async fn handle_mcp_command(args: McpCommandArgs) -> anyhow::Result<()> {
    info!("Starting MCP client");
    info!(prompt = %args.prompt, backend = %args.backend, "Initial configuration");

    // TODO: Implement MCP server connection and tool discovery
    // TODO: Create LLM backend adapter
    // TODO: Execute agentic loop

    let _ = (
        &args.model,
        &args.server,
        &args.server_args,
        args.max_tools_per_turn,
        args.verbose,
    );

    // Create MCP host
    let _host = McpHost::builder().build();

    info!("MCP client created");
    println!("MCP client command not yet fully implemented");
    println!("Prompt: {}", args.prompt);
    println!("Backend: {}", args.backend);
    println!("Max turns: {}", args.max_turns);

    Ok(())
}
