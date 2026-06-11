//! MCP client command handler.

use crate::cli::McpCommandArgs;
use tracing::{info, instrument};

/// Handle MCP client command.
#[instrument(skip_all, fields(backend = %args.backend, max_turns = args.max_turns))]
pub async fn handle_mcp_command(args: McpCommandArgs) -> anyhow::Result<()> {
    info!("Starting MCP client");
    info!(prompt = %args.prompt, backend = %args.backend, "Initial configuration");

    // TODO Phase 7: Implement via botticelli_mcp_client::BotticelliClient
    let _ = (
        &args.model,
        &args.server,
        &args.server_args,
        args.max_tools_per_turn,
        args.verbose,
    );

    info!("MCP client stub — awaiting Phase 7 rewrite");
    println!("MCP client not yet implemented (Phase 7)");
    println!("Prompt: {}", args.prompt);
    println!("Backend: {}", args.backend);
    println!("Max turns: {}", args.max_turns);

    Ok(())
}
