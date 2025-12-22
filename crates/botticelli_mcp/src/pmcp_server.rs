//! PMCP-based MCP server implementation.
//!
//! This is the new implementation using the pmcp SDK, which will eventually
//! replace the custom mcp-server implementation.

use crate::pmcp_adapters::McpToolAdapter;
use crate::tools::{
    CreateNarrativeTool, EchoTool, GenerateTool, ModifyNarrativeTool, SaveNarrativeTool,
    ServerInfoTool, ValidateNarrativeTool,
};
use anyhow::Result;
use pmcp::Server;
use std::sync::Arc;
use tracing::{info, instrument};

/// Registers all tools with the server builder.
/// 
/// This is the single source of truth for tool registration,
/// used by both stdio and HTTP servers.
#[instrument(skip(builder, db_ops))]
pub fn register_all_tools(
    mut builder: pmcp::ServerBuilder,
    #[cfg(feature = "database")] db_ops: Option<
        std::sync::Arc<dyn botticelli_interface::DatabaseRegistryOperations>,
    >,
) -> pmcp::ServerBuilder {
    tracing::info!("Starting tool registration");

    // Register core tools
    tracing::debug!("Registering core tools: echo, server_info, generate");
    builder = builder
        .tool("echo", McpToolAdapter::new(EchoTool))
        .tool("server_info", McpToolAdapter::new(ServerInfoTool))
        .tool("generate", McpToolAdapter::new(GenerateTool));
    tracing::info!("Registered 3 core tools");

    // TODO: Register ExportMetricsTool when metrics infrastructure is available

    // Register database tools (if feature enabled)
    #[cfg(feature = "database")]
    {
        use crate::tools::QueryContentTool;
        if let Some(ops) = db_ops {
            tracing::info!("Database feature enabled, registering query_content tool");
            builder = builder.tool(
                "query_content",
                McpToolAdapter::new(QueryContentTool::new(ops)),
            );
            tracing::info!("Registered database tool");
        } else {
            tracing::warn!("Database feature enabled but no DatabaseRegistryOperations provided");
        }
    }
    #[cfg(not(feature = "database"))]
    {
        tracing::debug!("Database feature not enabled, skipping database tools");
    }

    // Register narrative tools
    tracing::debug!("Registering narrative tools");
    builder = builder
        .tool("create_narrative", McpToolAdapter::new(CreateNarrativeTool))
        // StartNarrativeTool needs McpTool trait implementation - TODO
        .tool(
            "validate_narrative",
            McpToolAdapter::new(ValidateNarrativeTool),
        )
        .tool("save_narrative", McpToolAdapter::new(SaveNarrativeTool))
        .tool("modify_narrative", McpToolAdapter::new(ModifyNarrativeTool));
    tracing::info!("Registered 4 narrative tools");

    // Register elicitation tools
    {
        use crate::tools::{
            CreateNarrativeSessionTool, ElicitActTool, ElicitCarouselTool, ElicitMetadataTool,
            FinalizeNarrativeTool, GetNarrativeStateTool, ApplyValidationFixesTool,
            ValidateNarrativeSessionTool, NarrativeRegistry,
        };

        let registry = Arc::new(NarrativeRegistry::new());

        tracing::info!("Registering elicitation tools");
        builder = builder
            .tool(
                "create_narrative_session",
                McpToolAdapter::new(CreateNarrativeSessionTool::new(registry.clone())),
            )
            .tool(
                "elicit_metadata",
                McpToolAdapter::new(ElicitMetadataTool::new(registry.clone())),
            )
            .tool(
                "elicit_act",
                McpToolAdapter::new(ElicitActTool::new(registry.clone())),
            )
            .tool(
                "elicit_carousel",
                McpToolAdapter::new(ElicitCarouselTool::new(registry.clone())),
            )
            .tool(
                "get_narrative_state",
                McpToolAdapter::new(GetNarrativeStateTool::new(registry.clone())),
            )
            .tool(
                "validate_narrative_session",
                McpToolAdapter::new(ValidateNarrativeSessionTool::new(registry.clone())),
            )
            .tool(
                "apply_validation_fixes",
                McpToolAdapter::new(ApplyValidationFixesTool::new(registry.clone())),
            )
            .tool(
                "finalize_narrative",
                McpToolAdapter::new(FinalizeNarrativeTool::new(registry)),
            );
        tracing::info!("Registered 8 elicitation tools");
    }

    // Register ExecuteActTool (only when LLM features are enabled)
    #[cfg(any(
        feature = "gemini",
        feature = "anthropic",
        feature = "ollama",
        feature = "huggingface",
        feature = "groq"
    ))]
    {
        use crate::tools::ExecuteActTool;
        tracing::debug!("LLM feature enabled, registering execute_act tool");
        builder = builder.tool("execute_act", McpToolAdapter::new(ExecuteActTool::new()));
        tracing::info!("Registered execute_act tool");
    }
    #[cfg(not(any(
        feature = "gemini",
        feature = "anthropic",
        feature = "ollama",
        feature = "huggingface",
        feature = "groq"
    )))]
    {
        tracing::debug!("No LLM feature enabled, skipping execute_act tool");
    }

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
        tracing::debug!("LLM feature enabled, registering execute_narrative tool");
        builder = builder.tool(
            "execute_narrative",
            McpToolAdapter::new(ExecuteNarrativeTool::new()),
        );
        tracing::info!("Registered execute_narrative tool");
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
            DiscordBotCommandTool, DiscordGetChannelsTool, DiscordGetGuildInfoTool,
            DiscordGetMessagesTool, DiscordPostMessageTool, DiscordPostTool,
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

        // Optional Discord tools that need bot token
        if let Ok(token) = std::env::var("DISCORD_BOT_TOKEN") {
            match DiscordPostTool::new(token.clone()) {
                Ok(tool) => {
                    builder = builder.tool("discord_post", McpToolAdapter::new(tool));
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize DiscordPostTool: {}", e);
                }
            }

            match DiscordBotCommandTool::new(token) {
                Ok(tool) => {
                    builder = builder.tool("discord_bot_command", McpToolAdapter::new(tool));
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize DiscordBotCommandTool: {}", e);
                }
            }
        } else {
            tracing::info!(
                "DISCORD_BOT_TOKEN not set, skipping discord_post and discord_bot_command tools"
            );
        }

        // DiscordContentWorkflowTool requires ToolRegistry for orchestration
        // This creates a circular dependency - the workflow tool needs access to other tools
        // but we're still building the registry. This should be refactored to use
        // dependency injection or a two-phase initialization.
        tracing::info!(
            "Skipping discord_content_workflow tool (requires refactoring for tool dependencies)"
        );
    }

    tracing::info!("✅ Tool registration complete - all available tools registered");
    tracing::info!("Total tools registered: Core(3) + Narrative(4) + Elicitation(8) + feature-gated tools");
    builder
}

/// Runs the PMCP-based MCP server on stdio transport.
#[instrument(skip(db_ops))]
pub async fn run_pmcp_server(
    #[cfg(feature = "database")] db_ops: Option<
        std::sync::Arc<dyn botticelli_interface::DatabaseRegistryOperations>,
    >,
) -> Result<()> {
    info!("Starting PMCP-based MCP server");

    // Build server with all tools wrapped in adapters
    let builder = Server::builder()
        .name("botticelli-pmcp")
        .version(env!("CARGO_PKG_VERSION"))
        .capabilities(pmcp::types::capabilities::ServerCapabilities::tools_only());

    // Register all tools using shared registration function
    let builder = register_all_tools(
        builder,
        #[cfg(feature = "database")]
        db_ops,
    );

    // Build the server
    let server = builder.build()?;

    info!("Server built successfully, running on stdio");

    // Run on stdio transport
    server.run_stdio().await?;

    Ok(())
}
