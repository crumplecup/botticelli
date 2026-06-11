//! rmcp `ClientHandler` implementation for terminal I/O.

use rmcp::ErrorData;
use rmcp::handler::client::ClientHandler;
use rmcp::model::{
    ClientCapabilities, CreateMessageRequestParams, CreateMessageResult, Implementation,
    InitializeRequestParams, ListRootsResult, ProtocolVersion, SamplingContent, SamplingMessage,
    SamplingMessageContent,
};
use rmcp::service::{RequestContext, RoleClient};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::instrument;

/// rmcp `ClientHandler` that routes elicitation prompts to the terminal.
///
/// When the Botticelli server calls `create_message` (MCP sampling), this
/// handler prints the prompt and reads the human's answer from stdin.
#[derive(Clone, Debug, Default)]
pub struct TuiHandler;

impl TuiHandler {
    /// Creates a new terminal handler.
    #[instrument]
    pub fn new() -> Self {
        tracing::debug!("Creating TuiHandler");
        Self
    }
}

impl ClientHandler for TuiHandler {
    #[instrument(skip(self))]
    fn get_info(&self) -> InitializeRequestParams {
        let capabilities = ClientCapabilities::default();
        InitializeRequestParams::new(
            capabilities,
            Implementation::new("botticelli-tui", env!("CARGO_PKG_VERSION")),
        )
        .with_protocol_version(ProtocolVersion::V_2025_06_18)
    }

    #[instrument(skip(self, params, _context), fields(messages = params.messages.len()))]
    fn create_message(
        &self,
        params: CreateMessageRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> impl std::future::Future<Output = Result<CreateMessageResult, ErrorData>> + Send + '_ {
        async move {
            let prompt = params
                .messages
                .iter()
                .filter_map(|msg| match &msg.content {
                    SamplingContent::Single(SamplingMessageContent::Text(t)) => {
                        Some(t.text.clone())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            tracing::debug!(prompt_len = prompt.len(), "Received elicitation prompt");

            print!("{}\n> ", prompt);
            use std::io::Write as _;
            std::io::stdout()
                .flush()
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

            let mut stdin = BufReader::new(tokio::io::stdin());
            let mut line = String::new();
            stdin
                .read_line(&mut line)
                .await
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

            let answer = line.trim().to_string();
            tracing::debug!(answer_len = answer.len(), "User answered elicitation");

            Ok(CreateMessageResult::new(
                SamplingMessage::user_text(answer),
                "tui".to_string(),
            ))
        }
    }

    #[instrument(skip(self, _context))]
    fn list_roots(
        &self,
        _context: RequestContext<RoleClient>,
    ) -> impl std::future::Future<Output = Result<ListRootsResult, ErrorData>> + Send + '_ {
        tracing::debug!("Handling list_roots");
        std::future::ready(Ok(ListRootsResult::default()))
    }
}
