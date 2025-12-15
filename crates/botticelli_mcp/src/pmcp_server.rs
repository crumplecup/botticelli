//! PMCP-based MCP server implementation.
//!
//! This is the new implementation using the pmcp SDK, which will eventually
//! replace the custom mcp-server implementation.

use crate::pmcp_adapters::McpToolAdapter;
use crate::tools::{
    CreateNarrativeTool, EchoTool, ModifyNarrativeTool, SaveNarrativeTool, ServerInfoTool,
    ValidateNarrativeTool,
};
use anyhow::Result;
use pmcp::Server;
use tracing::{info, instrument};

/// Runs the PMCP-based MCP server.
#[instrument]
pub async fn run_pmcp_server() -> Result<()> {
    info!("Starting PMCP-based MCP server");

    // Build server with all tools wrapped in adapters
    let mut builder = Server::builder()
        .name("botticelli-pmcp")
        .version(env!("CARGO_PKG_VERSION"))
        .capabilities(pmcp::types::capabilities::ServerCapabilities::tools_only());

    // Register core tools
    builder = builder
        .tool("echo", McpToolAdapter::new(EchoTool))
        .tool("server_info", McpToolAdapter::new(ServerInfoTool));

    // Register database tools (if feature enabled)
    #[cfg(feature = "database")]
    {
        use crate::tools::QueryContentTool;
        builder = builder.tool("query_content", McpToolAdapter::new(QueryContentTool));
    }

    // Register narrative tools
    builder = builder
        .tool("create_narrative", McpToolAdapter::new(CreateNarrativeTool))
        .tool(
            "validate_narrative",
            McpToolAdapter::new(ValidateNarrativeTool),
        )
        .tool("save_narrative", McpToolAdapter::new(SaveNarrativeTool))
        .tool("modify_narrative", McpToolAdapter::new(ModifyNarrativeTool));

    // Register ExecuteNarrativeTool (only when LLM features are enabled)
    #[cfg(any(
        feature = "gemini",
        feature = "anthropic",
        feature = "ollama",
        feature = "huggingface",
        feature = "groq"
    ))]
    {
        use crate::tools::ExecuteNarrativeTool;
        builder = builder.tool(
            "execute_narrative",
            McpToolAdapter::new(ExecuteNarrativeTool::new()),
        );
    }

    // Register LLM tools (if features enabled)
    #[cfg(feature = "anthropic")]
    {
        use crate::tools::GenerateAnthropicTool;
        match GenerateAnthropicTool::new() {
            Ok(tool) => {
                builder = builder.tool("generate_anthropic", McpToolAdapter::new(tool));
            }
            Err(e) => {
                tracing::warn!("Failed to initialize GenerateAnthropicTool: {}", e);
            }
        }
    }

    #[cfg(feature = "gemini")]
    {
        use crate::tools::GenerateGeminiTool;
        match GenerateGeminiTool::new() {
            Ok(tool) => {
                builder = builder.tool("generate_gemini", McpToolAdapter::new(tool));
            }
            Err(e) => {
                tracing::warn!("Failed to initialize GenerateGeminiTool: {}", e);
            }
        }
    }

    #[cfg(feature = "ollama")]
    {
        use crate::tools::GenerateOllamaTool;
        match GenerateOllamaTool::new() {
            Ok(tool) => {
                builder = builder.tool("generate_ollama", McpToolAdapter::new(tool));
            }
            Err(e) => {
                tracing::warn!("Failed to initialize GenerateOllamaTool: {}", e);
            }
        }
    }

    #[cfg(feature = "groq")]
    {
        use crate::tools::GenerateGroqTool;
        match GenerateGroqTool::new() {
            Ok(tool) => {
                builder = builder.tool("generate_groq", McpToolAdapter::new(tool));
            }
            Err(e) => {
                tracing::warn!("Failed to initialize GenerateGroqTool: {}", e);
            }
        }
    }

    #[cfg(feature = "huggingface")]
    {
        use crate::tools::GenerateHuggingFaceTool;
        match GenerateHuggingFaceTool::new() {
            Ok(tool) => {
                builder = builder.tool("generate_huggingface", McpToolAdapter::new(tool));
            }
            Err(e) => {
                tracing::warn!("Failed to initialize GenerateHuggingFaceTool: {}", e);
            }
        }
    }

    // Register Discord tools (if feature enabled)
    #[cfg(feature = "discord")]
    {
        use crate::tools::{
            DiscordGetChannelsTool, DiscordGetGuildInfoTool, DiscordGetMessagesTool,
            DiscordPostMessageTool,
        };

        match DiscordGetChannelsTool::new() {
            Ok(tool) => {
                builder = builder.tool("discord_get_channels", McpToolAdapter::new(tool));
            }
            Err(e) => {
                tracing::warn!("Failed to initialize DiscordGetChannelsTool: {}", e);
            }
        }

        match DiscordGetMessagesTool::new() {
            Ok(tool) => {
                builder = builder.tool("discord_get_messages", McpToolAdapter::new(tool));
            }
            Err(e) => {
                tracing::warn!("Failed to initialize DiscordGetMessagesTool: {}", e);
            }
        }

        match DiscordGetGuildInfoTool::new() {
            Ok(tool) => {
                builder = builder.tool("discord_get_guild_info", McpToolAdapter::new(tool));
            }
            Err(e) => {
                tracing::warn!("Failed to initialize DiscordGetGuildInfoTool: {}", e);
            }
        }

        match DiscordPostMessageTool::new() {
            Ok(tool) => {
                builder = builder.tool("discord_post_message", McpToolAdapter::new(tool));
            }
            Err(e) => {
                tracing::warn!("Failed to initialize DiscordPostMessageTool: {}", e);
            }
        }
    }

    // Build the server
    let server = builder.build()?;

    info!("Server built successfully, running on stdio");

    // Run on stdio transport
    server.run_stdio().await?;

    Ok(())
}
