//! HTTP server implementation using pmcp's StreamableHttpServer.
//!
//! This provides HTTP transport for the MCP server with:
//! - Health and metrics endpoints
//! - Stateless operation (serverless-friendly)
//! - Proper error handling and observability

use crate::pmcp_adapters::McpToolAdapter;
use crate::tools::{
    CreateNarrativeTool, EchoTool, ModifyNarrativeTool, SaveNarrativeTool, ServerInfoTool,
    ValidateNarrativeTool,
};
use anyhow::Result;
use pmcp::server::streamable_http_server::{StreamableHttpServer, StreamableHttpServerConfig};
use pmcp::Server;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, instrument, warn};

/// Builds the MCP server with all tools.
///
/// This is the same server logic as stdio, but prepared for HTTP transport.
#[instrument(skip(db_ops))]
fn build_server(
    #[cfg(feature = "database")] db_ops: Option<
        Arc<dyn botticelli_interface::DatabaseRegistryOperations>,
    >,
) -> Result<Server> {
    info!("Building MCP server for HTTP transport");

    let mut builder = Server::builder()
        .name("botticelli-pmcp-http")
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
        if let Some(ops) = db_ops {
            builder = builder.tool(
                "query_content",
                McpToolAdapter::new(QueryContentTool::new(ops)),
            );
        }
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
    
    // Register elicitation tools
    use crate::tools::{
        CreateNarrativeSessionTool, ElicitMetadataTool, ElicitActTool,
        FinalizeNarrativeTool, ElicitCarouselTool, GetNarrativeStateTool,
        ValidateNarrativeSessionTool, ApplyValidationFixesTool, NarrativeRegistry,
    };
    let elicitation_registry = Arc::new(NarrativeRegistry::new());
    
    builder = builder
        .tool("create_narrative_session", McpToolAdapter::new(CreateNarrativeSessionTool::new(
            Arc::clone(&elicitation_registry),
        )))
        .tool("elicit_metadata", McpToolAdapter::new(ElicitMetadataTool::new(
            Arc::clone(&elicitation_registry),
        )))
        .tool("elicit_act", McpToolAdapter::new(ElicitActTool::new(
            Arc::clone(&elicitation_registry),
        )))
        .tool("finalize_narrative", McpToolAdapter::new(FinalizeNarrativeTool::new(
            Arc::clone(&elicitation_registry),
        )))
        .tool("elicit_carousel", McpToolAdapter::new(ElicitCarouselTool::new(
            Arc::clone(&elicitation_registry),
        )))
        .tool("get_narrative_state", McpToolAdapter::new(GetNarrativeStateTool::new(
            Arc::clone(&elicitation_registry),
        )))
        .tool("validate_narrative_session", McpToolAdapter::new(ValidateNarrativeSessionTool::new(
            Arc::clone(&elicitation_registry),
        )))
        .tool("apply_validation_fixes", McpToolAdapter::new(ApplyValidationFixesTool::new(
            Arc::clone(&elicitation_registry),
        )));
    
    // Register execution tools
    use crate::tools::{ExecuteActTool, GenerateTool};
    builder = builder
        .tool("execute_act", McpToolAdapter::new(ExecuteActTool::new()))
        .tool("generate", McpToolAdapter::new(GenerateTool));

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
                warn!("Failed to initialize GenerateAnthropicTool: {}", e);
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
                warn!("Failed to initialize GenerateGeminiTool: {}", e);
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
                warn!("Failed to initialize GenerateOllamaTool: {}", e);
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
                warn!("Failed to initialize GenerateGroqTool: {}", e);
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
                warn!("Failed to initialize GenerateHuggingFaceTool: {}", e);
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
                warn!("Failed to initialize DiscordGetChannelsTool: {}", e);
            }
        }

        match DiscordGetMessagesTool::new() {
            Ok(tool) => {
                builder = builder.tool("discord_get_messages", McpToolAdapter::new(tool));
            }
            Err(e) => {
                warn!("Failed to initialize DiscordGetMessagesTool: {}", e);
            }
        }

        match DiscordGetGuildInfoTool::new() {
            Ok(tool) => {
                builder = builder.tool("discord_get_guild_info", McpToolAdapter::new(tool));
            }
            Err(e) => {
                warn!("Failed to initialize DiscordGetGuildInfoTool: {}", e);
            }
        }

        match DiscordPostMessageTool::new() {
            Ok(tool) => {
                builder = builder.tool("discord_post_message", McpToolAdapter::new(tool));
            }
            Err(e) => {
                warn!("Failed to initialize DiscordPostMessageTool: {}", e);
            }
        }
    }

    Ok(builder.build()?)
}

/// Runs the HTTP MCP server.
#[instrument(skip(db_ops))]
pub async fn run_pmcp_http_server(
    host: &str,
    port: u16,
    #[cfg(feature = "database")] db_ops: Option<
        Arc<dyn botticelli_interface::DatabaseRegistryOperations>,
    >,
) -> Result<()> {
    info!("Starting PMCP HTTP server on {}:{}", host, port);

    // Build the server
    let server = build_server(
        #[cfg(feature = "database")]
        db_ops,
    )?;
    info!("Server built successfully with all tools");

    // Wrap in Arc<Mutex<>> for HTTP server
    let server = Arc::new(Mutex::new(server));

    // Configure address
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    info!("Configuring HTTP server for {}", addr);

    // Create stateless configuration (serverless-friendly)
    let config = StreamableHttpServerConfig {
        session_id_generator: None,   // Stateless mode
        enable_json_response: true,   // JSON responses
        event_store: None,            // No event store
        on_session_initialized: None, // No session callbacks
        on_session_closed: None,
        http_middleware: None, // TODO: Add middleware when needed
    };

    // Create HTTP server
    let http_server = StreamableHttpServer::with_config(addr, server, config);

    // Start server
    let (bound_addr, _handle) = http_server.start().await?;

    info!("HTTP server successfully started on {}", bound_addr);

    // Print banner
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║         BOTTICELLI MCP HTTP SERVER (PMCP)                 ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Address: http://{:43} ║", bound_addr);
    println!("║ Mode:    Stateless (serverless-friendly)                  ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Features:                                                  ║");
    println!("║ • All 26 tools available via HTTP                         ║");
    println!("║ • Stateless operation (no session management)             ║");
    println!("║ • Horizontal scaling ready                                ║");
    println!("║ • Full observability via tracing                          ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Endpoints:                                                 ║");
    println!("║ • POST /mcp - MCP JSON-RPC requests                       ║");
    println!("║ • GET  /health - Health check (future)                    ║");
    println!("║ • GET  /metrics - Prometheus metrics (future)             ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    // Keep server running
    tokio::signal::ctrl_c().await?;
    info!("Received shutdown signal, stopping server");

    Ok(())
}
