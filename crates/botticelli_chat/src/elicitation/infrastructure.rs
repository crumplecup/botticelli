//! Shared infrastructure for paradigm-based elicitation.
//!
//! This module provides common utilities for setting up MCP clients with
//! primitive elicitation tools, enabling the use of the elicitation crate's
//! derive macros across all elicitors.
//!
//! ## Migration Status
//!
//! This module is being migrated from pmcp to rmcp. The new rmcp-based
//! infrastructure uses compile-time tool registration via the `#[tool_handler]`
//! macro rather than runtime registration.

use crate::ElicitationDialog;
use botticelli_error::{ChatError, ChatErrorKind};
use botticelli_mcp::{BotticelliServer, DialogResource, InProcTransport};
use std::sync::Arc;
use tracing::{instrument, warn};

/// Create an MCP client connected to in-process server with primitive elicitation tools.
///
/// ## Migration Note
///
/// This function is temporarily disabled during the rmcp migration. The rmcp
/// model uses compile-time tool registration which requires restructuring how
/// elicitation tools are exposed.
///
/// TODO: Implement rmcp-based elicitation infrastructure that:
/// 1. Creates BotticelliServer with dialog resource
/// 2. Wraps server in InProcTransport for zero-overhead calls
/// 3. Provides elicitation-crate-compatible interface
#[instrument(skip(dialog))]
pub async fn create_mcp_client_for_dialog(
    _dialog: Box<dyn ElicitationDialog>,
) -> Result<InProcTransport, ChatError> {
    warn!("create_mcp_client_for_dialog temporarily disabled during rmcp migration");
    
    // Temporary: Create a basic server without dialog integration
    // This allows compilation but elicitation won't work until properly implemented
    let server = BotticelliServer::builder().build();
    
    // TODO: Wrap server in InProcTransport and return
    // For now, return error to make intent clear
    Err(ChatError::new(ChatErrorKind::InvalidState(
        "Elicitation infrastructure not yet migrated to rmcp".to_string(),
    )))
}

