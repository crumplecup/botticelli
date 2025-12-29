//! Shared infrastructure for paradigm-based elicitation.
//!
//! This module provides common utilities for setting up MCP clients with
//! primitive elicitation tools, enabling the use of the elicitation crate's
//! derive macros across all elicitors.

use crate::ElicitationDialog;
use botticelli_error::{ChatError, ChatErrorKind};
use botticelli_mcp::{DialogResource, InProcTransport, register_all_tools};
use pmcp::{Client, ClientCapabilities, Server};
use std::sync::Arc;
use tracing::instrument;

/// Create an MCP client connected to in-process server with primitive elicitation tools.
///
/// This helper function sets up the infrastructure needed for the elicitation
/// crate's derive macros to work:
/// 1. Wraps the dialog in DialogResource
/// 2. Creates MCP server with primitive elicitation tools
/// 3. Sets up InProcTransport for zero-overhead communication
/// 4. Returns initialized MCP client
///
/// # Example
///
/// ```no_run
/// use botticelli_chat::elicitation::infrastructure::create_mcp_client_for_dialog;
/// use botticelli_chat::elicitation::NarrativeMetadata;
/// use elicitation::Elicitation;
///
/// async fn example(dialog: Box<dyn ElicitationDialog>) {
///     let client = create_mcp_client_for_dialog(dialog).await.unwrap();
///     let metadata = NarrativeMetadata::elicit(&client).await.unwrap();
/// }
/// ```
#[instrument(skip(dialog))]
pub async fn create_mcp_client_for_dialog(
    dialog: Box<dyn ElicitationDialog>,
) -> Result<Client<InProcTransport>, ChatError> {
    // Wrap dialog in resource for sharing across tools
    let dialog_resource = Arc::new(DialogResource::new(dialog));

    // Build MCP server with primitive elicitation tools
    let builder = Server::builder()
        .name("narrative-elicitation")
        .version(env!("CARGO_PKG_VERSION"))
        .capabilities(pmcp::types::capabilities::ServerCapabilities::tools_only());

    let builder = register_all_tools(
        builder,
        Some(dialog_resource),
        #[cfg(feature = "database")]
        None,
    );

    let server = builder.build().map_err(|e| {
        ChatError::new(ChatErrorKind::InvalidState(format!(
            "Failed to build MCP server: {}",
            e
        )))
    })?;

    // Create in-process transport (zero-copy channel communication)
    let (client_transport, server_transport) = InProcTransport::pair();
    let _server_handle = InProcTransport::spawn_server(server, server_transport);

    // Initialize MCP client
    let mut client = Client::new(client_transport);
    client
        .initialize(ClientCapabilities::minimal())
        .await
        .map_err(|e| {
            ChatError::new(ChatErrorKind::InvalidState(format!(
                "Failed to initialize MCP client: {}",
                e
            )))
        })?;

    Ok(client)
}
